//! One compact record per committed state and per final certificate entry.
use crate::Rejection;
use serde::{Deserialize, Serialize};
/// A verified search commit or a reproduced failed closure removal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum StepOutcome {
    /// Full-checker validated state.
    Accepted,
    /// Full or structural certificate rejection.
    Rejected {
        #[doc = "Actual admission failure."]
        reason: Rejection,
    },
}
/// Versioned deterministic wire log. Intervals are inclusive original unit IDs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StepLog {
    /// qlrs-reduce-log/v1.
    pub schema: String,
    /// Consecutive record number.
    pub step: u64,
    /// input, search or certificate.
    pub phase: String,
    /// RLE removed units relative to the preceding commit or final kept set.
    pub removed: Vec<[u32; 2]>,
    /// Units left in the candidate.
    pub kept_units: usize,
    /// Observed events left in the candidate.
    pub kept_events: usize,
    /// Reserved full-check slots as of writing this record.
    pub calls_full: u64,
    /// Numerical queries as of writing this record.
    pub probes_fast: u64,
    /// Admission or rejection.
    pub outcome: StepOutcome,
}
