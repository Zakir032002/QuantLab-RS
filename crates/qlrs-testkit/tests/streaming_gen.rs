//! Streaming keeps the established generator bytes and propagates sink failures.
#![forbid(unsafe_code)]
#![allow(clippy::unwrap_used)]
use std::io::{self, Write};

#[test]
fn scale_003_rng_vectors_and_legacy_stream_commitment() {
    let vectors: Vec<(u64, [u64; 16])> =
        qlrs_io::decode(include_bytes!("../fixtures/rng-v1.json")).unwrap();
    for (seed, expected) in vectors {
        let mut rng = qlrs_testkit::rng::Rng::new(seed);
        assert_eq!(
            std::array::from_fn::<_, 16, _>(|_| rng.next_u64()),
            expected
        );
    }
    let mut bytes = Vec::new();
    qlrs_testkit::generate_into(20_000, 42, None, &mut bytes).unwrap();
    // Input produced and committed by SHA256 at CP3.1 before this refactor.
    assert_eq!(
        qlrs_io::digest(&bytes),
        "563dffc8ea4f60531504245d3943b0276cd920bcaa97bff2107a1a343aa2649a"
    );
}

struct Broken;
impl Write for Broken {
    fn write(&mut self, _: &[u8]) -> io::Result<usize> {
        Err(io::Error::other("injected sink failure"))
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
#[test]
fn scale_003_both_generators_propagate_writer_failure() {
    assert!(
        qlrs_testkit::generate_into(20, 1, None, Broken)
            .unwrap_err()
            .contains("injected sink failure")
    );
    let config = qlrs_testkit::GeneratorConfig::profile("interleaved", 20, 1).unwrap();
    assert!(
        qlrs_testkit::generate_rich_into(config, None, Broken)
            .unwrap_err()
            .contains("injected sink failure")
    );
}

#[test]
fn hard_006_atomic_failure_preserves_target_and_unowned_files() {
    let dir = std::env::temp_dir().join(format!("qlrs-atomic-{}", std::process::id()));
    std::fs::create_dir(&dir).unwrap();
    let target = dir.join("output");
    std::fs::write(&target, b"old").unwrap();
    let unowned = dir.join(format!(".qlrs.tmp.{}.0", std::process::id()));
    std::fs::write(&unowned, b"unowned").unwrap();
    assert!(qlrs_io::atomic_file(&target, |file| file.write_all(b"new")).is_err());
    assert_eq!(std::fs::read(&unowned).unwrap(), b"unowned");
    let failure = qlrs_io::atomic_file(&target, |file| {
        file.write_all(b"partial")?;
        Err::<(), _>(io::Error::other("injected writer failure"))
    });
    assert!(failure.is_err());
    assert_eq!(std::fs::read(&target).unwrap(), b"old");
    assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 2);
    qlrs_io::atomic_file(&target, |file| file.write_all(b"complete")).unwrap();
    assert_eq!(std::fs::read(&target).unwrap(), b"complete");
    std::fs::remove_dir_all(dir).unwrap();
}
