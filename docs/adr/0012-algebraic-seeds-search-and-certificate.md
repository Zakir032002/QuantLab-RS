# ADR-0012: algebraic seeds, search and certificates

Context: Stage A hard-pinned every contributing fill. That made search and most
minimality passes vacuous. The latest user instruction requires minimal pins
and real removable candidates, while preserving exact target identity.

Decision: pin only the observed opening claim and target unit. Lifecycle suffixes
remain atomic. Conditional FIFO prerequisites use checkpoint inventory first.
Contributor requirements are checked by exact residual equality, rather than
hard claim-to-every-fill edges that would make all target contributors pinned.
Every admitted candidate also passes scoped full Checker/model/lifecycle checks,
anti-new-key admission and final full re-verification. This implements the
rebased real-search requirement without losing the required I2 fill evidence.

Default SBR starts from nonzero per-unit contributions plus pinned prerequisites.
A rejected seed uses deterministic cost-sorted prefix unions with O(1) checked
residual queries. ddmin-closures is retained as a selectable reference. Small
financial windows (at most 18 unpinned units) additionally enumerate numerical
subsets in Gray order with O(1) residual/cost updates. Only cheaper exact-residual,
closed candidates reach the full oracle. This removes balanced buy/sell groups
that a single-removal pass cannot find. No global-minimum promise is made for
larger windows.

I1 fast arithmetic is used only under Exact rounding; other rounding and I3
request a full oracle. Checked overflow also falls back. A fast value never
admits a witness by itself. Differential testing compares full-checker residuals
on closed subsets, including shared-prerequisite prefix unions.

A final repeated closure-removal pass builds a certificate with graph commitment,
source-ID mapping, predicate version, exact target, original keys, inclusive RLE
closures and actual full-oracle rejection reasons. Pinned-boundary closures have
structural KeyMissing proofs. candidates_tested counts actual removable-root
tests on the final kept set; zero is explicitly vacuous. Exhaustion returns a
partial certificate and closure_1_minimal=false. Independent bundle rechecking
rebuilds the graph and reruns every listed removal with the full checker.

Do not assume acceptance is monotone: ledger preconditions and anti-new checks
can invalidate supersets. Full-validated commits and the final local certificate
supply soundness independently of that assumption.

Compatibility: reduce/reduce_with_limits and Predicate::test signatures remain.
Add Algorithm, reduce_with_options, FastPredicate and certificate/stat fields.
Keep --time-limit-ms and --max-calls; candidate caps conservatively include cache
hits. Input/context/final validation are mandatory outside the search cap.
No fingerprint encoding change: differing reduced event patterns already produce
different class fingerprints under fp1, as in Part 1.
