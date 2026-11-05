//! QuantLab-RS command-line auditor for synthetic Mode T traces.
#![forbid(unsafe_code)]
use clap::{Args, Parser, Subcommand};
pub use qlrs_io::Limits;
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
        #[command(flatten)]
        input: InputOptions,
        #[arg(long)]
        checkpoint: Option<PathBuf>,
        #[arg(long)]
        partial: bool,
    },
    /// Reduce a complete violation into a new, verified evidence directory.
    Reduce {
        trace: PathBuf,
        #[command(flatten)]
        input: InputOptions,
        #[arg(short, long)]
        output: PathBuf,
        #[arg(long)]
        checkpoint: Option<PathBuf>,
        #[arg(long)]
        target_seq: Option<u64>,
        /// Cooperative search time limit in milliseconds.
        #[arg(long, default_value_t = 30_000)]
        time_limit_ms: u64,
        /// Maximum candidate predicate calls, including cache hits.
        #[arg(long, default_value_t = 10_000)]
        max_calls: u64,
        /// sbr (default), or ddmin-closures for reference comparisons.
        #[arg(long, default_value = "sbr", value_parser = ["sbr", "ddmin-closures"])]
        algorithm: String,
    },
    /// Verify an existing evidence bundle offline.
    Bundle {
        #[command(subcommand)]
        command: BundleCommand,
    },
}
#[derive(Subcommand)]
enum BundleCommand {
    Verify {
        directory: PathBuf,
        #[arg(long)]
        recheck_certificate: bool,
        #[arg(long)]
        max_entries: Option<usize>,
        #[arg(long)]
        max_entry_bytes: Option<u64>,
    },
}
#[derive(Args, Default)]
struct InputOptions {
    /// Enable the proposed Part 3 defaults (pending default-verdict review).
    #[arg(long)]
    strict_limits: bool,
    #[arg(long)]
    max_line_bytes: Option<usize>,
    #[arg(long)]
    max_interned: Option<usize>,
    #[arg(long)]
    max_events: Option<usize>,
    #[arg(long)]
    max_input_bytes: Option<u64>,
    /// Estimated allocation budget in bytes.
    #[arg(long)]
    memory_budget: Option<u64>,
}
impl InputOptions {
    fn read(&self, path: PathBuf) -> Result<qlrs_io::ParsedTrace, qlrs_io::InputError> {
        if !self.strict_limits
            && self.max_line_bytes.is_none()
            && self.max_interned.is_none()
            && self.max_events.is_none()
            && self.max_input_bytes.is_none()
            && self.memory_budget.is_none()
        {
            return qlrs_io::ParsedTrace::read(path);
        }
        let defaults = Limits::default();
        qlrs_io::ParsedTrace::read_with_limits(
            path,
            Limits {
                max_line_bytes: self.max_line_bytes.unwrap_or(defaults.max_line_bytes),
                max_interned: self.max_interned.unwrap_or(defaults.max_interned),
                max_events: self.max_events.unwrap_or(defaults.max_events),
                max_input_bytes: self.max_input_bytes.unwrap_or(defaults.max_input_bytes),
                memory_budget_bytes: self.memory_budget,
                ..defaults
            },
        )
    }
}
struct CliError {
    code: u8,
    message: String,
    json: Option<Vec<u8>>,
}
impl From<String> for CliError {
    fn from(message: String) -> Self {
        Self {
            code: 1,
            message,
            json: None,
        }
    }
}
impl From<qlrs_io::InputError> for CliError {
    fn from(error: qlrs_io::InputError) -> Self {
        Self {
            code: error.kind.exit_code(),
            message: error.to_string(),
            json: qlrs_io::encode(&error).ok(),
        }
    }
}
fn checkpoint(path: Option<PathBuf>) -> Result<Option<SyntheticCheckpoint>, String> {
    path.map(|path| {
        std::fs::read(path)
            .map_err(|e| e.to_string())
            .and_then(|bytes| qlrs_io::decode(&bytes).map_err(|e| e.to_string()))
    })
    .transpose()
}
fn run(cli: Cli) -> Result<u8, CliError> {
    match cli.command {
        Command::Verify {
            trace,
            input,
            checkpoint: cp,
            partial,
        } => {
            let cp = checkpoint(cp)?;
            let mut trace = input.read(trace)?;
            qlrs_ledger::Ledger::prepare_with_checkpoint(&mut trace.store, cp.as_ref())?;
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
            input,
            output,
            checkpoint: cp,
            target_seq,
            time_limit_ms,
            max_calls,
            algorithm,
        } => {
            let cp = checkpoint(cp)?;
            let mut trace = input.read(trace)?;
            qlrs_ledger::Ledger::prepare_with_checkpoint(&mut trace.store, cp.as_ref())?;
            let reduction = qlrs_reduce::reduce_with_options(
                &trace.store,
                &qlrs_io::digest(&trace.source),
                cp.as_ref(),
                target_seq,
                qlrs_reduce::ReductionLimits {
                    max_calls,
                    time_limit: std::time::Duration::from_millis(time_limit_ms),
                },
                if algorithm == "sbr" {
                    qlrs_reduce::Algorithm::Sbr
                } else {
                    qlrs_reduce::Algorithm::DdminClosures
                },
            )?;
            let manifest = qlrs_bundle::write(output, &trace, &reduction)?;
            io::stdout()
                .lock()
                .write_all(&qlrs_io::encode(&manifest).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
            Ok(0)
        }
        Command::Bundle {
            command:
                BundleCommand::Verify {
                    directory,
                    recheck_certificate,
                    max_entries,
                    max_entry_bytes,
                },
        } => {
            let manifest = if max_entries.is_none() && max_entry_bytes.is_none() {
                qlrs_bundle::verify_with_options(directory, recheck_certificate)?
            } else {
                let defaults = Limits::default();
                qlrs_bundle::verify_with_limits(
                    directory,
                    recheck_certificate,
                    Limits {
                        max_entries: max_entries.unwrap_or(defaults.max_entries),
                        max_entry_bytes: max_entry_bytes.unwrap_or(defaults.max_entry_bytes),
                        ..defaults
                    },
                )?
            };
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
            eprintln!("qlrs: {}", error.message);
            if let Some(json) = error.json {
                let _ = io::stdout().lock().write_all(&json);
            }
            ExitCode::from(error.code)
        }
    }
}
