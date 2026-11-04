//! Generate deterministic interleaved/sparse/multi-instrument fixtures.
#![forbid(unsafe_code)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let count = args
        .first()
        .ok_or("usage: generate_rich ORDERS OUTPUT PROFILE [D01-D11] [INDEX]")?
        .parse()?;
    let output = args.get(1).ok_or("missing output")?;
    let profile = args.get(2).ok_or("missing profile")?;
    let mutation = args
        .get(3)
        .map(|id| qlrs_testkit::Defect::parse(id).ok_or("unknown defect"))
        .transpose()?
        .map(|defect| {
            args.get(4)
                .map_or(Ok(count / 2), |value| value.parse())
                .map(|index| (defect, index))
        })
        .transpose()?;
    let fixture = qlrs_testkit::generate_rich(
        qlrs_testkit::GeneratorConfig::profile(profile, count, 42)?,
        mutation,
    )?;
    std::fs::write(output, fixture.bytes)?;
    Ok(())
}
