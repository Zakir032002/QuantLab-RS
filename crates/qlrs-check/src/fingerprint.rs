//! Canonical, versioned fingerprint encoding and its protected test vector.
#![forbid(unsafe_code)]
use crate::Invariant;
use qlrs_schema::Kind;
/// Canonical fingerprint bytes: see SPEC.md for the versioned field encoding.
#[must_use]
pub fn fingerprint_bytes(
    invariant: Invariant,
    code: &str,
    pattern: &[(Kind, u64)],
    sign: i8,
    relation: &str,
) -> Vec<u8> {
    let mut bytes = Vec::new();
    let mut string = |text: &str| {
        bytes.extend_from_slice(&u32::try_from(text.len()).unwrap_or(u32::MAX).to_le_bytes());
        bytes.extend_from_slice(text.as_bytes());
    };
    string(invariant.name());
    string(code);
    let mut counts = [0_u64; 8];
    for &(kind, count) in pattern {
        if let Some(total) = counts.get_mut(kind as usize) {
            *total = total.saturating_add(count);
        }
    }
    let entries = counts.iter().filter(|count| **count > 0).count();
    bytes.extend_from_slice(&u32::try_from(entries).unwrap_or(u32::MAX).to_le_bytes());
    for (kind, count) in counts
        .into_iter()
        .enumerate()
        .filter(|(_, count)| *count > 0)
    {
        bytes.push(u8::try_from(kind).unwrap_or(u8::MAX));
        bytes.extend_from_slice(&count.to_le_bytes());
    }
    bytes.extend_from_slice(&sign.to_le_bytes());
    bytes.extend_from_slice(
        &u32::try_from(relation.len())
            .unwrap_or(u32::MAX)
            .to_le_bytes(),
    );
    bytes.extend_from_slice(relation.as_bytes());
    bytes
}
/// Class-level heuristic hash, distinct from the exact reduction instance key.
#[must_use]
pub fn class_fingerprint(
    invariant: Invariant,
    code: &str,
    pattern: &[(Kind, u64)],
    sign: i8,
    relation: &str,
) -> String {
    format!(
        "qlrs-fp1:{}",
        blake3::hash(&fingerprint_bytes(invariant, code, pattern, sign, relation)).to_hex()
    )
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::arithmetic_side_effects)]
mod tests {
    use super::*;
    #[test]
    fn req_fp_001_canonical_vector_and_order_independence() {
        let pattern = [(Kind::Filled, 1), (Kind::CashClaim, 1)];
        let bytes = fingerprint_bytes(Invariant::I1, "cash-conservation", &pattern, -1, "-fee");
        let mut hex = String::new();
        for byte in &bytes {
            use std::fmt::Write as _;
            write!(hex, "{byte:02x}").unwrap();
        }
        assert_eq!(
            hex,
            include_str!("../../../fixtures/golden/fingerprint-vector.hex").trim()
        );
        assert_eq!(
            class_fingerprint(Invariant::I1, "cash-conservation", &pattern, -1, "-fee"),
            "qlrs-fp1:d36cbaabf8cae916908640e54736cc3856466a66ffc4e3d37f08f9f55d373a72"
        );
        assert_eq!(
            class_fingerprint(Invariant::I1, "cash-conservation", &pattern, -1, "-fee"),
            class_fingerprint(
                Invariant::I1,
                "cash-conservation",
                &[(Kind::CashClaim, 1), (Kind::Filled, 1)],
                -1,
                "-fee"
            )
        );
        assert_eq!(Invariant::I1.name(), "I1");
        assert_eq!(Invariant::I2.name(), "I2");
        assert_eq!(Invariant::I3.name(), "I3");
    }
    #[test]
    fn req_fp_003_all_fields_have_distinct_canonical_encodings() {
        let base = fingerprint_bytes(Invariant::I1, "cash", &[(Kind::Filled, 3)], -1, "-fee");
        for other in [
            fingerprint_bytes(Invariant::I2, "cash", &[(Kind::Filled, 3)], -1, "-fee"),
            fingerprint_bytes(Invariant::I1, "cash2", &[(Kind::Filled, 3)], -1, "-fee"),
            fingerprint_bytes(Invariant::I1, "cash", &[(Kind::Accepted, 3)], -1, "-fee"),
            fingerprint_bytes(Invariant::I1, "cash", &[(Kind::Filled, 4)], -1, "-fee"),
            fingerprint_bytes(Invariant::I1, "cash", &[(Kind::Filled, 3)], 1, "-fee"),
            fingerprint_bytes(Invariant::I1, "cash", &[(Kind::Filled, 3)], -1, "+fee"),
        ] {
            assert_ne!(base, other);
        }
    }
    #[test]
    fn req_fp_004_utf8_lengths_sign_and_zero_multiset_entries() {
        let bytes = fingerprint_bytes(Invariant::I2, "é", &[(Kind::Filled, 1)], -1, "x");
        let expected = vec![
            2, 0, 0, 0, b'I', b'2', 2, 0, 0, 0, 0xc3, 0xa9, 1, 0, 0, 0, 3, 1, 0, 0, 0, 0, 0, 0, 0,
            255, 1, 0, 0, 0, b'x',
        ];
        assert_eq!(bytes, expected);
        assert_eq!(
            fingerprint_bytes(Invariant::I2, "é", &[(Kind::Filled, 1)], -1, "x"),
            fingerprint_bytes(
                Invariant::I2,
                "é",
                &[(Kind::CashClaim, 0), (Kind::Filled, 1), (Kind::Filled, 0)],
                -1,
                "x"
            )
        );
        assert_eq!(
            fingerprint_bytes(Invariant::I2, "x", &[(Kind::Filled, 4)], 0, "other"),
            fingerprint_bytes(
                Invariant::I2,
                "x",
                &[(Kind::Filled, 1), (Kind::Filled, 3)],
                0,
                "other"
            )
        );
    }
}
