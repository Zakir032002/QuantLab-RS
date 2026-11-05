# Part 3 default-limit review (R9)

Part 3 explicitly requests the following limits, but R9 separately requires
approval when a formerly accepted input receives a different verdict/exit.
These are proposed mappings, not claimed executions of unimplemented code.
Executed before results and minimal balance examples are in
evidence/part3/cp3.0/current-behaviour/.

| Existing input | Executed current exit | Proposed default result |
|---|---:|---|
| Valid trace with >1 MiB leading whitespace on its first line | 0 | 1, E-LIMIT-LINE-BYTES |
| Starting balance with 41 decimal digits | 3, numeric overflow | 1, E-LIMIT-NUMBER-LEN |
| Starting balance with 19 fractional digits | 4, unsupported scale | 1, E-SCALE |
| Otherwise valid overlong identifier/string | Currently uncapped | 1, E-LIMIT-STRING |

The request is to enable the documented section 7 default resource limits
(line, JSON depth, literal length, string length, input size, events/interned
counts) and the E-SCALE classification. Resource-limit ceilings are configurable
within the specified hard bounds. Opt-in memory budgets return INCONCLUSIVE.

Checked arithmetic overflow within the literal-length limit stays INCONCLUSIVE;
instrument precision/model mismatches stay UNSUPPORTED. Unknown event kinds
retain SPEC's UNSUPPORTED treatment. Known-schema unknown/duplicate keys and
invalid UTF-8 remain input errors, gaining stable identifiers. Existing golden
files, tolerances, catalogue statuses and fingerprint bytes remain protected.
