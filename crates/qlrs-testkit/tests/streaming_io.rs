//! Source non-retention, exact replay bytes and deterministic chunk admission.
#![forbid(unsafe_code)]
#![allow(clippy::unwrap_used)]
use qlrs_io::{Limits, ParsedTrace, SourceReader};

#[test]
fn scale_002_file_offsets_chunks_and_builder_match_memory() {
    for shape in qlrs_testkit::SHAPES {
        let fixture =
            qlrs_testkit::generate_shape(shape, 3, 1000, Some(qlrs_testkit::Defect::D06)).unwrap();
        let memory = ParsedTrace::parse(fixture.bytes.clone()).unwrap();
        let path = std::env::temp_dir().join(format!("qlrs-source-{}-{shape}", std::process::id()));
        std::fs::write(&path, &fixture.bytes).unwrap();
        for chunk in [1, 1024, 64 << 10, 8 << 20] {
            let trace = ParsedTrace::read_chunked(&path, Some(Limits::default()), chunk).unwrap();
            assert_eq!(trace.source, [] as [u8; 0]);
            assert_eq!(trace.lines, []);
            assert_eq!(trace.store, memory.store);
            assert_eq!(trace.source_digest(), memory.source_digest());
            assert_eq!(trace.stats().offsets_bytes, trace.line_count() * 12);
            let mut bytes = Vec::new();
            trace
                .write_kept(
                    &qlrs_schema::AliveMask::all(trace.store.units.len()),
                    &mut bytes,
                )
                .unwrap();
            assert_eq!(bytes, fixture.bytes);
            for event in [0, trace.store.len() / 2, trace.store.len() - 1] {
                let reference = trace.source_ref(event).unwrap();
                assert_eq!(
                    trace.read_record(&reference).unwrap(),
                    fixture
                        .bytes
                        .get(memory.line_range(event + 1).unwrap())
                        .unwrap()
                );
            }
        }
        let mut builder =
            qlrs_io::TraceStoreBuilder::with_limits(memory.store.header.clone(), Limits::default())
                .unwrap();
        for (line, bytes) in fixture
            .bytes
            .split_inclusive(|b| *b == b'\n')
            .enumerate()
            .skip(1)
        {
            builder.push_json(bytes, line + 1).unwrap();
        }
        assert_eq!(builder.finish(), memory.store);
        std::fs::remove_file(path).unwrap();
    }
}

#[test]
fn hard_001_source_modification_and_truncation_are_errors() {
    use std::io::{Seek, SeekFrom, Write};
    let path = std::env::temp_dir().join(format!("qlrs-source-changed-{}", std::process::id()));
    let bytes = include_bytes!("../../../fixtures/clean/C01-worked-example.jsonl");
    std::fs::write(&path, bytes).unwrap();
    let trace = ParsedTrace::read(&path).unwrap();
    let reference = trace.source_ref(0).unwrap();
    let mut file = std::fs::OpenOptions::new().write(true).open(&path).unwrap();
    file.seek(SeekFrom::Start(reference.offset)).unwrap();
    file.write_all(b" ").unwrap();
    assert_eq!(
        trace.read_record(&reference).unwrap_err().code,
        "E-INPUT-CHANGED"
    );
    file.set_len(1).unwrap();
    assert_eq!(
        trace.read_record(&reference).unwrap_err().code,
        "E-INPUT-CHANGED"
    );
    std::fs::remove_file(path).unwrap();
}

#[test]
fn hard_002_spooling_is_bounded_and_replays_original_bytes() {
    let bytes = include_bytes!("../../../fixtures/clean/C01-worked-example.jsonl");
    for limit in [bytes.len() - 1, bytes.len(), bytes.len() + 1] {
        let result = ParsedTrace::spool(
            bytes.as_slice(),
            Some(Limits {
                max_input_bytes: limit as u64,
                ..Limits::default()
            }),
            1024,
        );
        assert_eq!(result.is_ok(), limit >= bytes.len());
        if let Ok(trace) = result {
            assert_eq!(trace.source, [] as [u8; 0]);
            assert_eq!(trace.source_digest(), qlrs_io::digest(bytes));
        } else {
            assert_eq!(result.unwrap_err().code, "E-LIMIT-INPUT-BYTES");
        }
    }
}
