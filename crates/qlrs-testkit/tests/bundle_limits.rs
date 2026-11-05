//! Resource and confinement boundaries exercise the real offline verifier.
#![forbid(unsafe_code)]
#![allow(clippy::unwrap_used)]
use qlrs_io::{Limits, ParsedTrace};
use std::{
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

fn bundle() -> PathBuf {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let path = std::env::temp_dir().join(format!(
        "qlrs-bundle-limits-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let fixture = qlrs_testkit::generate(4, 42, Some((qlrs_testkit::Defect::D03, 2))).unwrap();
    let mut trace = ParsedTrace::parse(fixture.bytes).unwrap();
    qlrs_check::Checker::prepare(&mut trace.store);
    let reduced = qlrs_reduce::reduce(
        &trace.store,
        &trace.source_digest(),
        None,
        fixture.target_seq,
    )
    .unwrap();
    qlrs_bundle::write(&path, &trace, &reduced).unwrap();
    path
}

#[test]
fn hard_002_bundle_count_and_byte_boundaries() {
    let path = bundle();
    let count = std::fs::read_dir(&path).unwrap().count();
    let bytes = std::fs::read_dir(&path)
        .unwrap()
        .map(|e| e.unwrap().metadata().unwrap().len())
        .max()
        .unwrap();
    for limit in [count - 1, count, count + 1] {
        let result = qlrs_bundle::verify_with_limits(
            &path,
            false,
            Limits {
                max_entries: limit,
                ..Limits::default()
            },
        );
        assert_eq!(result.is_ok(), limit >= count);
        if let Err(error) = result {
            assert!(error.starts_with("E-BUNDLE-ENTRIES"));
        }
    }
    for limit in [bytes - 1, bytes, bytes + 1] {
        let result = qlrs_bundle::verify_with_limits(
            &path,
            false,
            Limits {
                max_entry_bytes: limit,
                ..Limits::default()
            },
        );
        assert_eq!(result.is_ok(), limit >= bytes);
        if let Err(error) = result {
            assert!(error.starts_with("E-BUNDLE-ENTRY-BYTES"));
        }
    }
    std::fs::remove_dir_all(path).unwrap();
}

#[test]
fn hard_005_bundle_access_stays_inside_root_and_rejects_symlinks() {
    let path = bundle();
    let mut reads = 0;
    qlrs_bundle::verify_with_observer(&path, true, Limits::default(), |file| {
        assert_eq!(file.parent(), Some(path.as_path()));
        reads += 1;
    })
    .unwrap();
    assert!(reads > 0);
    let link = path.with_extension("link");
    std::os::unix::fs::symlink(&path, &link).unwrap();
    assert!(
        qlrs_bundle::verify(&link)
            .unwrap_err()
            .starts_with("E-BUNDLE-PATH")
    );
    std::fs::remove_file(link).unwrap();
    let manifest = path.join("manifest.json");
    std::fs::remove_file(&manifest).unwrap();
    std::os::unix::fs::symlink("/etc/passwd", manifest).unwrap();
    qlrs_bundle::verify_with_observer(&path, false, Limits::default(), |_| {
        panic!("read before rejecting symlink")
    })
    .unwrap_err();
    std::fs::remove_dir_all(path).unwrap();
}

#[test]
fn hard_002_index_boundary_uses_reserved_sentinel() {
    assert_eq!(
        qlrs_schema::checked_index(4_294_967_293).unwrap(),
        4_294_967_293
    );
    assert_eq!(
        qlrs_schema::checked_index(4_294_967_294).unwrap(),
        4_294_967_294
    );
    for value in [4_294_967_295, 4_294_967_296] {
        assert_eq!(qlrs_schema::checked_index(value), Err("U-LIMIT-INDEX"));
    }
}

#[test]
fn hard_005_replacing_a_checked_file_with_a_symlink_cannot_escape() {
    let path = bundle();
    let outside = path.with_extension("canary");
    std::fs::write(&outside, b"outside bytes must never be read").unwrap();
    let mut swapped = false;
    let error = qlrs_bundle::verify_with_observer(&path, false, Limits::default(), |file| {
        if file.file_name().is_some_and(|name| name == "REPRODUCE.md") && !swapped {
            std::fs::remove_file(file).unwrap();
            std::os::unix::fs::symlink(&outside, file).unwrap();
            swapped = true;
        }
    })
    .unwrap_err();
    assert!(swapped);
    assert!(error.starts_with("E-BUNDLE-PATH"), "{error}");
    std::fs::remove_dir_all(path).unwrap();
    std::fs::remove_file(outside).unwrap();
}

#[test]
fn hard_005_replacing_the_root_path_does_not_redirect_open_descriptors() {
    let path = bundle();
    let moved = path.with_extension("moved");
    let outside = path.with_extension("outside");
    std::fs::create_dir(&outside).unwrap();
    std::fs::write(outside.join("manifest.json"), b"outside invalid bytes").unwrap();
    let mut swapped = false;
    qlrs_bundle::verify_with_observer(&path, false, Limits::default(), |_| {
        if !swapped {
            std::fs::rename(&path, &moved).unwrap();
            std::os::unix::fs::symlink(&outside, &path).unwrap();
            swapped = true;
        }
    })
    .unwrap();
    std::fs::remove_file(path).unwrap();
    std::fs::remove_dir_all(moved).unwrap();
    std::fs::remove_dir_all(outside).unwrap();
}
