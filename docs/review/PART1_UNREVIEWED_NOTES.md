# Part 1 review against the rebased source

Reviewed schema, numeric representation, bundle integrity, CLI, and invariant
checks before changing these interfaces. This is an agent review, not human
sign-off. Multi-instrument headers, per-instrument positions and key scopes are
already supported; S5 requires no header extension.

Numeric: checked integer arithmetic, exact decimal parsing and wide intermediates
are retained unchanged. No new numeric policy or dependency is required.
Schema: orders/trades already carry checkpoint lifecycle/filled quantity context;
add a version field rather than renaming these. Existing interned order lookup
handles cross-namespace symbol ordering. No reimplementation is needed.
Checker: the existing incremental pattern counts and local I2 address most Stage
A findings. Missing-evidence precedence contradicts the rebased requirement;
CP2.0 corrects it. A dynamic iterator and repeated order-limit rescaling remain
performance work, not correctness fixes.
Reducer: existing D04 size regressions pass before new fixes. The old global
claim frontier couples unrelated scopes; the new I2 regression fails before
CP2.0. Initial local-fill pins make minimality frequently vacuous: Stage B must
replace them with algebraic evidence checks and report tested candidates.
Bundle: canonical manifests, whitelisted paths and content hashes already reject
tampering. V1 scoped semantics need explicit roles, certificates and offline
rechecking before Part 2 release. V0 reading must remain available.
CLI: current budget flags already implement best-so-far. Preserve their names;
add algorithm selection and certificate recheck rather than renaming flags.

Raw before regressions are in bench/results/CP2.0/before/regressions.txt: D04
sizes pass; precedence and unrelated I2 evidence fail. No already-passing Stage A
test has been marked ignored to manufacture a failure history. Existing R2 tail,
R3 local-window and carried-position tests remain active. R4 timings and R5
helper mutation evidence will be recorded with the respective implementation.
