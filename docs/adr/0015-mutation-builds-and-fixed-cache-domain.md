# ADR-0015: mutation build caching and the fixed cache domain

Use unoptimized, no-debug-info test builds for the complete mutation campaign.
All tests and mutation scopes remain selected; overflow/debug assertions remain
enabled. Release flags and performance measurements do not change. The separate
normal-profile suite still runs before completion.

Cargo verbose output exposed a build-script problem in source archives and
mutation copies: watching a missing .git/HEAD dirtied the bundle and its
consumers on every invocation. Resolve real HEAD/refs paths with git rev-parse
and watch only existing paths, plus build.rs itself. This also supports Git
worktrees. Embedded source/compiler values are unchanged. The first campaign
attempt was interrupted for excessive rebuild cost; raw partial results are
explicitly marked incomplete. A build-only correction was applied identically
to existing copies and recorded before the later restart.

The public cache-window bug hypothesis was refuted. AliveMask::intersection
already fixes observation bounds to the receiver, so public Predicate::test
normalizes callers safely. The initially failing assertion expected the wrong
public contract and is not a correctness-bug reproduction. The final regression
checks normalization and a defensive private-batch guard; private batch callers
bypass normalization and must preserve the predicate's fixed domain. Signature,
public acceptance, fingerprint bytes and numeric behavior are unchanged.

No incomplete attempt contributes to the final mutation score. Raw attempts,
corrected reasoning and the final full enumeration are retained. Mutation floors
remain separate for numeric/checker, reducer and fingerprint code.
