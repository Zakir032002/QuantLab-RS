# Acceptance is not monotone

fixtures/part2/nonmonotone-extra-order.jsonl is a saved counterexample. Target is
the cash claim at original seq 7. A keeps O0 and that claim. B additionally keeps
the NOISE lifecycle, whose second acceptance at seq 5 is illegal. Both sets are
closed and A is a subset of B. Their financial contributions match; the
lifecycle-validity requirement makes P(A)=true and P(B)=false.

req_reduce_017 checks this and verifies seed recovery. Therefore the full
predicate has no monotonicity guarantee. SBR commits only full-checked witnesses;
closure certificates prove a local minimum relative to boundary pins, without
relying on monotonicity. ddmin-closures and exhaustive small-window checks remain
comparison tools. This is a genuine counterexample, not a suppressed property.
