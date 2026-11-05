# ADR-0017: bounded input checks and confined bundle I/O

Status: implemented as explicit opt-in; default-verdict review pending R9.

The old parser accepts arbitrarily long whitespace lines, reports numeric
capacity as INCONCLUSIVE, and reports excess scale as UNSUPPORTED. The concrete
before/after inputs are in [PART3_VERDICT_REVIEW](../PART3_VERDICT_REVIEW.md).
Changing those defaults requires the requested user approval.

`qlrs_io::Limits` separates malformed input (`E-*`, exit 1), model capacity
(`U-*`, exit 4), and an explicit memory budget (`I-*`, exit 3). The defaults and
ceilings are the section 7 table in PART_03_INSTRUCTIONS. `--strict-limits` or an
explicit resource flag enables these checks; ordinary invocations preserve the
existing policy pending review. `parse_with_limits` and `read_with_limits`
expose the same opt-in interface. A bounded preflight detects decoded duplicate
keys and resource limits; serde_json remains the grammar and record parser.
This is not a replacement scanner or the optional F04 parser experiment.

Bundle reads pass through one validated file-access function. The verifier
rejects a symlink root, symlink/non-regular entries, non-UTF-8 names and manifest
paths outside its filename allowlist, before reading their content. Explicit
bundle limits bound entries and bytes. An observer records every content read
for the confinement oracle. File writes use the shared sibling-temp/fsync/rename
helper. A bundle is verified in its own staging directory and renamed only
after completion; a failed write removes only its owned staging directory.

The filesystem is assumed not to be concurrently replaced by another process.
Safe standard-library path operations do not provide an openat/O_NOFOLLOW
capability guarantee against a hostile directory-renaming race. This limitation
must remain explicit in the security review, rather than claiming such a proof.
