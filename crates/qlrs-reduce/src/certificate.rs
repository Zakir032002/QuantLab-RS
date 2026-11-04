//! Portable closure certificates; numeric unit IDs are accompanied by source IDs.
use qlrs_check::InstanceKey;
use serde::{Deserialize, Serialize};
/// Why removing a kept dependency closure is rejected.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Rejection {
    /// A different numerical residual at the target claim.
    ResidualChanged {
        #[doc = "Raw integer at the target scale."]
        new_residual: String,
    },
    /// A contradiction absent from the original input.
    NewViolation {
        #[doc = "Previously absent exact key."]
        key: InstanceKey,
    },
    /// A lifecycle suffix became illegal.
    LifecycleInvalid {
        #[doc = "Lifecycle violation code."]
        sub_code: String,
    },
    /// Target assertion or required observed opening is missing.
    KeyMissing,
}
/// One original unit, mapped by its first retained source sequence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnitIdentity {
    /// Unit ID in the original input.
    pub id: u32,
    /// First retained event in this unit.
    pub first_original_seq: u64,
}
/// One failed removable-root test. Intervals are inclusive.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClosureTest {
    /// Original root unit ID.
    pub root: u32,
    /// Sorted run-length encoded removed units.
    pub removed_closure: Vec<[u32; 2]>,
    /// Replay rejection.
    pub outcome: Rejection,
    /// `full` includes model/lifecycle checks, `fast` is a linear rejection.
    pub evaluator: String,
}
/// Independently recheckable proof, complete or explicitly partial on exhaustion.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Certificate {
    /// Wire format identifier.
    pub schema: String,
    /// Full admission semantics identifier.
    pub predicate_version: String,
    /// Exact target.
    pub target: InstanceKey,
    /// Original contradiction keys, sorted and deduplicated.
    pub original_keys: Vec<InstanceKey>,
    /// Commitment to compact CSR edges expressed in source sequence IDs.
    pub graph_digest: String,
    /// Original-unit to source-sequence mapping.
    pub kept_units: Vec<UnitIdentity>,
    /// Only the observed opening boundary and target unit are pinned.
    pub pinned: Vec<u32>,
    /// Failed closure candidates in deterministic root order.
    pub entries: Vec<ClosureTest>,
    /// Number tested for this final retained set; not a guessed boolean.
    pub candidates_tested: u64,
    /// True exactly when no removable closure was tested.
    pub vacuous: bool,
    /// Exhaustive coverage of removable roots without an exhausted budget.
    pub complete: bool,
}
/// Encode a sorted unit list without expanding long consecutive ranges.
#[must_use]
pub fn rle(units: &[usize]) -> Vec<[u32; 2]> {
    let mut out: Vec<[u32; 2]> = Vec::new();
    for unit in units {
        let Ok(unit) = u32::try_from(*unit) else {
            continue;
        };
        if let Some(last) = out.last_mut()
            && last.get(1).copied().and_then(|end| end.checked_add(1)) == Some(unit)
        {
            if let Some(end) = last.get_mut(1) {
                *end = unit;
            }
        } else {
            out.push([unit, unit]);
        }
    }
    out
}
