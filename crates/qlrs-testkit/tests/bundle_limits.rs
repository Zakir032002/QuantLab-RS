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
        &qlrs_io::digest(&trace.source),
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
