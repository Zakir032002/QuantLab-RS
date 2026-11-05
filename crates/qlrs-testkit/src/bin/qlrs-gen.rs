//! Streaming synthetic traces; buffers and open orders stay bounded.
#![forbid(unsafe_code)]
use clap::Parser;
use qlrs_testkit::{
    Defect, GeneratorConfig, generate_into, generate_rich_into, rng::Rng, shape_config,
};
use serde::Serialize;
use std::{
    fs::File,
    io::{self, BufWriter, Write},
    path::PathBuf,
};

#[derive(Parser)]
struct Args {
    #[arg(long, default_value = "s1m", value_parser = ["s1m", "s10m", "s100m"])]
    preset: String,
    #[arg(long, default_value_t = 1)]
    seed: u64,
    #[arg(long, default_value = "simple", value_parser = ["simple", "interleaved", "long-lived", "sparse", "S1", "S2", "S3", "S4", "S5", "S6", "S7", "S8", "S9"])]
    profile: String,
    #[arg(long)]
    out: PathBuf,
    #[arg(long)]
    allow_huge: bool,
    #[arg(long)]
    defect: Option<String>,
    #[arg(long, default_value = "middle", value_parser = ["early", "middle", "late"])]
    at: String,
}
struct DigestWriter<'a> {
    file: &'a mut File,
    hash: blake3::Hasher,
    bytes: u64,
}
impl Write for DigestWriter<'_> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let count = self.file.write(bytes)?;
        let bytes = bytes
            .get(..count)
            .ok_or_else(|| io::Error::other("invalid write length"))?;
        self.hash.update(bytes);
        self.bytes = self
            .bytes
            .checked_add(u64::try_from(count).map_err(io::Error::other)?)
            .ok_or_else(|| io::Error::other("byte count overflow"))?;
        Ok(count)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.file.flush()
    }
}
#[derive(Serialize)]
struct Metadata<'a> {
    generator_version: &'static str,
    preset: &'a str,
    profile: &'a str,
    seed: u64,
    requested_events: usize,
    events: u64,
    bytes: u64,
    blake3: String,
    qlrs_commit: &'static str,
    target_seq: Option<u64>,
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let requested_events = match args.preset.as_str() {
        "s1m" => 1_000_000,
        "s10m" => 10_000_000,
        _ if args.allow_huge => 100_000_000,
        _ => return Err("s100m requires --allow-huge".into()),
    };
    let simple = matches!(args.profile.as_str(), "simple" | "S1");
    let orders = requested_events / if simple { 5 } else { 4 };
    let seed = Rng::new(args.seed).next_u64();
    let index = match args.at.as_str() {
        "early" => orders / 100,
        "late" => orders - orders / 200,
        _ => orders / 2,
    };
    let defect = args
        .defect
        .as_deref()
        .map(|text| Defect::parse(text).ok_or("unknown defect"))
        .transpose()?;
    let mutation = defect.map(|defect| (defect, index));
    let meta = qlrs_io::atomic_file(&args.out, |file| {
        let mut writer = BufWriter::with_capacity(
            1 << 20,
            DigestWriter {
                file,
                hash: blake3::Hasher::new(),
                bytes: 0,
            },
        );
        let (target_seq, events) = if simple {
            generate_into(orders, seed, mutation, &mut writer)
        } else {
            let config = if args.profile.starts_with('S') {
                shape_config(&args.profile, seed, orders)
            } else {
                GeneratorConfig::profile(&args.profile, orders, seed)
            }
            .map_err(io::Error::other)?;
            generate_rich_into(config, mutation, &mut writer)
        }
        .map_err(io::Error::other)?;
        writer.flush()?;
        let writer = writer
            .into_inner()
            .map_err(io::IntoInnerError::into_error)?;
        Ok(Metadata {
            generator_version: "qlrs-gen/v1",
            preset: &args.preset,
            profile: &args.profile,
            seed: args.seed,
            requested_events,
            events,
            bytes: writer.bytes,
            blake3: writer.hash.finalize().to_hex().to_string(),
            qlrs_commit: qlrs_bundle::source_commit(),
            target_seq,
        })
    })?;
    let mut meta_path = args.out.as_os_str().to_owned();
    meta_path.push(".meta.json");
    let bytes = qlrs_io::encode(&meta)?;
    qlrs_io::atomic_file(PathBuf::from(meta_path), |file| file.write_all(&bytes))?;
    io::stdout().lock().write_all(&bytes)?;
    Ok(())
}
