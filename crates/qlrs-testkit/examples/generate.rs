//! Generate a reproducible synthetic trace without adding a CLI runtime edge.
#![forbid(unsafe_code)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let count = args
        .first()
        .ok_or("usage: generate ORDERS OUTPUT [D01..D14] [INDEX]")?
        .parse()?;
    let path = args.get(1).ok_or("missing output path")?;
    let mutation = args
        .get(2)
        .map(|d| qlrs_testkit::Defect::parse(d).ok_or("unknown defect"))
        .transpose()?
        .map(|d| {
            args.get(3)
                .map_or(Ok(count / 2), |n| n.parse())
                .map(|index| (d, index))
        })
        .transpose()?;
    let fixture = qlrs_testkit::generate(count, 42, mutation)?;
    std::fs::write(path, fixture.bytes)?;
    Ok(())
}
