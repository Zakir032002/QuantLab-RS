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
        /// Use only the deterministic candidate-call budget.
        #[arg(long)]
        no_time_limit: bool,
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
    /// Chunk size in bytes; records are never split during parsing.
    #[arg(long, default_value_t = 8 << 20)]
    io_chunk_bytes: usize,
    /// Reject stdin and pipes instead of copying to an owned temporary file.
    #[arg(long)]
    no_spool: bool,
    /// Include retained-memory statistics in the JSON result.
    #[arg(long)]
    stats: bool,
    /// The `serde_json` reference remains the product parser.
    #[arg(long, default_value = "reference", value_parser = ["reference"])]
    parser: String,
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
        let enabled = self.strict_limits
            || self.max_line_bytes.is_some()
            || self.max_interned.is_some()
            || self.max_events.is_some()
            || self.max_input_bytes.is_some()
            || self.memory_budget.is_some();
        let defaults = Limits::default();
        let limits = enabled.then_some(Limits {
            max_line_bytes: self.max_line_bytes.unwrap_or(defaults.max_line_bytes),
            max_interned: self.max_interned.unwrap_or(defaults.max_interned),
            max_events: self.max_events.unwrap_or(defaults.max_events),
            max_input_bytes: self.max_input_bytes.unwrap_or(defaults.max_input_bytes),
            memory_budget_bytes: self.memory_budget,
            ..defaults
        });
        let stdin = path.as_os_str() == "-";
        let regular = !stdin && std::fs::metadata(&path).is_ok_and(|m| m.is_file());
        if regular {
            if limits.is_none() && self.io_chunk_bytes == 8 << 20 {
                return qlrs_io::ParsedTrace::read(path);
            }
            return qlrs_io::ParsedTrace::read_chunked(path, limits, self.io_chunk_bytes);
        }
        if self.no_spool {
            return Err(qlrs_io::InputError {
                kind: qlrs_io::ErrorKind::Input,
                code: "E-INPUT-NOT-SEEKABLE",
                line: 0,
                message: "input requires spooling".into(),
            });
        }
        if stdin {
            qlrs_io::ParsedTrace::spool(io::stdin().lock(), limits, self.io_chunk_bytes)
        } else {
            let file = std::fs::File::open(path).map_err(|e| qlrs_io::InputError {
                kind: qlrs_io::ErrorKind::Input,
                code: "E-IO",
                line: 0,
                message: e.to_string(),
            })?;
            qlrs_io::ParsedTrace::spool(file, limits, self.io_chunk_bytes)
        }
    }
}

struct CliError {
    code: u8,
    message: String,
    json: Option<Vec<u8>>,
}
impl From<String> for CliError {
    fn from(message: String) -> Self {
        for code in [
            "E-BUNDLE-PATH",
            "E-BUNDLE-ENTRIES",
            "E-BUNDLE-ENTRY-BYTES",
            "U-LIMIT-INDEX",
        ] {
            if message
                .strip_prefix(code)
                .is_some_and(|tail| tail.is_empty() || tail.starts_with(':'))
            {
                return qlrs_io::InputError {
                    kind: if code.starts_with("U-") {
                        qlrs_io::ErrorKind::Unsupported
                    } else {
                        qlrs_io::ErrorKind::Input
                    },
                    code,
                    line: 0,
                    message: message
                        .trim_start_matches(code)
                        .trim_start_matches(':')
                        .trim()
                        .to_owned(),
                }
                .into();
            }
        }
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
fn with_stats(mut bytes: Vec<u8>, stats: Option<qlrs_io::TraceStats>) -> Result<Vec<u8>, String> {
    if let Some(stats) = stats {
        bytes.pop();
        let stats = qlrs_io::encode(&stats).map_err(|e| e.to_string())?;
        let mut out = b"{\"report\":".to_vec();
        out.extend_from_slice(&bytes);
        out.extend_from_slice(b",\"stats\":");
        out.extend_from_slice(stats.strip_suffix(b"\n").unwrap_or(&stats));
        out.extend_from_slice(b"}\n");
        bytes = out;
    }
    Ok(bytes)
}
fn print_json(bytes: &[u8]) -> Result<(), String> {
    io::stdout()
        .lock()
        .write_all(bytes)
        .map_err(|e| e.to_string())
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
            let bytes = with_stats(
                qlrs_io::encode(&report).map_err(|e| e.to_string())?,
                input.stats.then(|| trace.stats()),
            )?;
            print_json(&bytes)?;
            Ok(report.status.exit_code())
        }
        Command::Reduce {
            trace,
            input,
            output,
            checkpoint: cp,
            target_seq,
            time_limit_ms,
            no_time_limit,
            max_calls,
            algorithm,
        } => {
            let cp = checkpoint(cp)?;
            let mut trace = input.read(trace)?;
            qlrs_ledger::Ledger::prepare_with_checkpoint(&mut trace.store, cp.as_ref())?;
            let reduction = qlrs_reduce::reduce_with_options(
                &trace.store,
                &trace.source_digest(),
                cp.as_ref(),
                target_seq,
                qlrs_reduce::ReductionLimits {
                    max_calls,
                    time_limit: if no_time_limit {
                        std::time::Duration::MAX
                    } else {
                        std::time::Duration::from_millis(time_limit_ms)
                    },
                },
                if algorithm == "sbr" {
                    qlrs_reduce::Algorithm::Sbr
                } else {
                    qlrs_reduce::Algorithm::DdminClosures
                },
            )?;
            let manifest = qlrs_bundle::write(output, &trace, &reduction)?;
            let bytes = with_stats(
                qlrs_io::encode(&manifest).map_err(|e| e.to_string())?,
                input.stats.then(|| trace.stats()),
            )?;
            print_json(&bytes)?;
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
            print_json(&qlrs_io::encode(&manifest).map_err(|e| e.to_string())?)?;
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
