# Replay this evidence

From this bundle directory, using qlrs 0.1.0:

```sh
qlrs verify reduced_trace.jsonl --checkpoint synthetic_checkpoint.json
qlrs bundle verify .
```

The first command must return VIOLATION (exit 2); the second must exit 0.
The synthetic checkpoint, when present, is independently computed expected
state replacing removed history. It is not an observed engine claim.
This is a Mode T output witness, not an engine-input reproduction.
Minimality: closure-1-minimal; not a global minimum. Fingerprint relations are hints, not diagnoses.
