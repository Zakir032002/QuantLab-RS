//! Seed raw parser fuzzing from every existing shape and catalogue operator.
#![forbid(unsafe_code)]
fn main() -> Result<(), String> {
    for shape in qlrs_testkit::SHAPES {
        for defect in
            std::iter::once(None).chain(qlrs_testkit::DEFECTS.into_iter().take(11).map(Some))
        {
            let fixture = qlrs_testkit::generate_shape(shape, 3, 64, defect)?;
            put("io_trace", &fixture.bytes)?;
            for line in fixture.bytes.split_inclusive(|b| *b == b'\n') {
                put("io_line", line)?;
            }
        }
    }
    Ok(())
}
fn put(target: &str, bytes: &[u8]) -> Result<(), String> {
    let root = std::path::Path::new("fuzz/corpus").join(target);
    std::fs::create_dir_all(&root).map_err(|e| e.to_string())?;
    std::fs::write(root.join(qlrs_io::digest(bytes)), bytes).map_err(|e| e.to_string())
}
