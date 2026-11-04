//! Golden-file BLAKE3 manifest generator. Does not modify the goldens.
#![forbid(unsafe_code)]
use std::path::{Path, PathBuf};
fn walk(
    root: &Path,
    path: &Path,
    out: &mut Vec<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        if kind.is_symlink() {
            return Err("golden symlinks are forbidden".into());
        }
        if kind.is_dir() {
            walk(root, &entry.path(), out)?;
        } else if entry.file_name() != "MANIFEST.blake3" {
            out.push(entry.path().strip_prefix(root)?.into());
        }
    }
    Ok(())
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let arg = std::env::args().nth(1).ok_or("usage: hash DIRECTORY")?;
    let root = Path::new(&arg);
    let mut paths = Vec::new();
    walk(root, root, &mut paths)?;
    paths.sort();
    for path in paths {
        println!(
            "{}  {}",
            qlrs_io::digest(&std::fs::read(root.join(&path))?),
            path.to_string_lossy().replace('\\', "/")
        );
    }
    Ok(())
}
