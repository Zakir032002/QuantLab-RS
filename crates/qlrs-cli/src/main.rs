//! QuantLab-RS command-line auditor for synthetic Mode T traces.
#![forbid(unsafe_code)]
use clap::{Parser, Subcommand};
use qlrs_schema::{AliveMask, SyntheticCheckpoint};
use std::{
    io::{self, Write},
    path::PathBuf,
    process::ExitCode,
};
#[derive(Parser)]
#[command(
    name = "qlrs",
    version,
    about = "Exact financial trace verification and lifecycle-valid counterexamples"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}
#[derive(Subcommand)]
enum Command {
    /// Verify a synthetic qlrs-trace/v0 JSONL file. Outputs deterministic JSON.
    Verify {
        trace: PathBuf,
        #[arg(long)]
        checkpoint: Option<PathBuf>,
        #[arg(long)]
        partial: bool,
    },
    /// Reduce a complete violation into a new, verified evidence directory.
    Reduce {
        trace: PathBuf,
        #[arg(short, long)]
        output: PathBuf,
        #[arg(long)]
        checkpoint: Option<PathBuf>,
        #[arg(long)]
        target_seq: Option<u64>,
    },
    /// Verify an existing evidence bundle offline.
    Bundle {
        #[command(subcommand)]
        command: BundleCommand,
    },
}
#[derive(Subcommand)]
enum BundleCommand {
    Verify { directory: PathBuf },
}
fn checkpoint(path: Option<PathBuf>) -> Result<Option<SyntheticCheckpoint>, String> {
    path.map(|path| {
        std::fs::read(path)
            .map_err(|e| e.to_string())
            .and_then(|bytes| qlrs_io::decode(&bytes).map_err(|e| e.to_string()))
    })
    .transpose()
}
fn run(cli: Cli) -> Result<u8, String> {
    match cli.command {
        Command::Verify {
            trace,
            checkpoint: cp,
            partial,
        } => {
            let cp = checkpoint(cp)?;
            let mut trace = qlrs_io::ParsedTrace::read(trace).map_err(|e| e.to_string())?;
            qlrs_ledger::Ledger::prepare(&mut trace.store);
            if let Some(cp) = &cp {
                qlrs_ledger::Ledger::initial(&trace.store, Some(cp))?;
            }
            let report = qlrs_check::Checker::check(
                &trace.store,
                &AliveMask::all(trace.store.units.len()),
                cp.as_ref(),
                partial,
            );
            io::stdout()
                .lock()
                .write_all(&qlrs_io::encode(&report).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
            Ok(report.status.exit_code())
        }
        Command::Reduce {
            trace,
            output,
            checkpoint: cp,
            target_seq,
        } => {
            let cp = checkpoint(cp)?;
            let mut trace = qlrs_io::ParsedTrace::read(trace).map_err(|e| e.to_string())?;
            qlrs_ledger::Ledger::prepare(&mut trace.store);
            let reduction = qlrs_reduce::reduce(
                &trace.store,
                &qlrs_io::digest(&trace.source),
                cp.as_ref(),
                target_seq,
            )?;
            let manifest = qlrs_bundle::write(output, &trace, &reduction)?;
            io::stdout()
                .lock()
                .write_all(&qlrs_io::encode(&manifest).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
            Ok(0)
        }
        Command::Bundle {
            command: BundleCommand::Verify { directory },
        } => {
            let manifest = qlrs_bundle::verify(directory)?;
            io::stdout()
                .lock()
                .write_all(&qlrs_io::encode(&manifest).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
            Ok(0)
        }
    }
}
fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(code) => ExitCode::from(code),
        Err(error) => {
            eprintln!("qlrs: {error}");
            ExitCode::from(1)
        }
    }
}
