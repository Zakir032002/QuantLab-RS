//! Boundaries exercise strict opt-in APIs while the default R9 review is pending.
#![forbid(unsafe_code)]
#![allow(clippy::unwrap_used)]
use qlrs_io::{ErrorKind, Limits, ParsedTrace, validate_line};
const CLEAN: &[u8] = include_bytes!("../../../fixtures/clean/C01-worked-example.jsonl");

#[test]
fn hard_003_decoded_keys_depth_strings_numbers_and_unicode() {
    for (bytes, code) in [
        (br#"{"seq":1,"\u0073eq":2}"#.as_slice(), "E-DUPLICATE-KEY"),
        (br#"{"custom":1,"custom":2}"#.as_slice(), "E-DUPLICATE-KEY"),
        (br#"{"a":{"seq":1,"seq":2}}"#.as_slice(), "E-DUPLICATE-KEY"),
        (br#""\ud800""#.as_slice(), "E-UTF8"),
        (b"\xff".as_slice(), "E-UTF8"),
    ] {
        assert_eq!(
            validate_line(bytes, 7, Limits::default()).unwrap_err().code,
            code
        );
    }
    // The same key is legal in distinct objects, and escaped delimiters do not
    // change the nesting count or become object keys.
    validate_line(
        br#"[{"seq":1},{"seq":2,"reason":"\"{x}"}]"#,
        1,
        Limits::default(),
    )
    .unwrap();
    let long_number = format!("{{\"starting_balance\":\"{}\"}}", "9".repeat(1_000_000));
    assert_eq!(
        validate_line(long_number.as_bytes(), 1, Limits::default())
            .unwrap_err()
            .code,
        "E-LIMIT-NUMBER-LEN"
    );
    for size in [3_usize, 4, 5] {
        let limits = Limits {
            max_string_bytes: 4,
            max_numeric_literal_len: 4,
            max_json_depth: 4,
            ..Limits::default()
        };
        let string = format!("\"{}\"", "x".repeat(size));
        let number = "9".repeat(size);
        let depth = format!("{}0{}", "[".repeat(size), "]".repeat(size));
        for input in [string.as_bytes(), number.as_bytes(), depth.as_bytes()] {
            assert_eq!(validate_line(input, 1, limits).is_ok(), size <= 4);
        }
        let decimal = format!("{{\"starting_balance\":\"0.{}\"}}", "0".repeat(size));
        assert_eq!(
            validate_line(
                decimal.as_bytes(),
                1,
                Limits {
                    max_decimal_scale: 4,
                    ..Limits::default()
                }
            )
            .is_ok(),
            size <= 4
        );
    }
    for length in [15, 16, 17] {
        assert_eq!(
            validate_line(
                &vec![b' '; length],
                1,
                Limits {
                    max_line_bytes: 16,
                    ..Limits::default()
                }
            )
            .is_ok(),
            length <= 16
        );
    }
    validate_line(
        br#""\u00e9""#,
        1,
        Limits {
            max_string_bytes: 2,
            ..Limits::default()
        },
    )
    .unwrap();
    assert_eq!(
        validate_line(
            br#""\u00e9""#,
            1,
            Limits {
                max_string_bytes: 1,
                ..Limits::default()
            }
        )
        .unwrap_err()
        .code,
        "E-LIMIT-STRING"
    );
}

#[test]
fn hard_002_input_event_interner_and_budget_boundaries() {
    let check = |limits| ParsedTrace::parse_with_limits(CLEAN.to_vec(), limits);
    let clean = check(Limits::default()).unwrap();
    assert_eq!(clean.store.strings.len(), 2);
    assert_eq!(clean.store.len(), 5);
    for (limits, code, kind) in [
        (
            Limits {
                max_events: 4,
                ..Limits::default()
            },
            "U-LIMIT-EVENTS",
            ErrorKind::Unsupported,
        ),
        (
            Limits {
                max_interned: 1,
                ..Limits::default()
            },
            "U-LIMIT-INTERNED",
            ErrorKind::Unsupported,
        ),
        (
            Limits {
                max_input_bytes: u64::try_from(CLEAN.len()).unwrap() - 1,
                ..Limits::default()
            },
            "E-LIMIT-INPUT-BYTES",
            ErrorKind::Input,
        ),
        (
            Limits {
                memory_budget_bytes: Some(0),
                ..Limits::default()
            },
            "I-BUDGET-MEMORY",
            ErrorKind::Budget,
        ),
        (
            Limits {
                max_json_depth: 9,
                ..Limits::default()
            },
            "E-LIMIT-CONFIG",
            ErrorKind::Input,
        ),
    ] {
        let error = check(limits).unwrap_err();
        assert_eq!((error.code, error.kind), (code, kind));
    }
    for extra in [0, 1] {
        check(Limits {
            max_events: 5 + extra,
            max_interned: 2 + extra,
            max_input_bytes: u64::try_from(CLEAN.len() + extra).unwrap(),
            ..Limits::default()
        })
        .unwrap();
    }
    let estimate = u64::try_from(CLEAN.len() * 2 + 6 * 160).unwrap() + (64 << 20);
    for budget in [estimate - 1, estimate, estimate + 1] {
        assert_eq!(
            check(Limits {
                memory_budget_bytes: Some(budget),
                ..Limits::default()
            })
            .is_ok(),
            budget >= estimate
        );
    }
}

#[test]
fn hard_001_two_gib_line_is_rejected_after_bounded_read() {
    use std::io::{Seek, SeekFrom, Write};
    let path = std::env::temp_dir().join(format!("qlrs-oversized-{}", std::process::id()));
    let mut file = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&path)
        .unwrap();
    file.seek(SeekFrom::Start((2_u64 << 30) - 1)).unwrap();
    file.write_all(b"\n").unwrap();
    drop(file);
    let error = ParsedTrace::read_with_limits(&path, Limits::default()).unwrap_err();
    std::fs::remove_file(path).unwrap();
    assert_eq!(error.code, "E-LIMIT-LINE-BYTES");
    assert_eq!(error.line, 1);
}
