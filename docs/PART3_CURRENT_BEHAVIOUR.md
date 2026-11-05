# Current behaviour before Part 3

Reference: 9b9c335, Rust 1.99.1. Executed cases and exact CLI outputs are recorded
in evidence/part3/cp3.0/current-behaviour/results.json, with companion run metadata.
No product code had changed during these probes.

| Input | Current exit / behaviour |
|---|---|
| Clean worked example | 0, CONSISTENT |
| Invalid UTF-8 | 1, source line error |
| Missing terminal newline | 1; currently reports line 1 |
| CRLF | Accepted, same verdict as LF |
| Unknown header/event key | 1 |
| Duplicate known header/event key | 1 |
| Deep nested array in place of header | 1 |
| Leading whitespace longer than the proposed line cap | Accepted; no line limit today |
| Decimal with 19 fractional digits | 4, UNSUPPORTED |
| Million-digit starting balance | 3, INCONCLUSIVE numeric overflow |
| Exponent or leading plus in a decimal | 1 |
| Leading-zero decimal | 4, UNSUPPORTED noncanonical decimal |

Input errors are presently message strings rather than stable identifiers.
Numeric overflow remains INCONCLUSIVE according to SPEC; syntax-invalid
overflow remains an input error. Unknown event kinds with valid common fields
remain UNSUPPORTED, not malformed-input errors. Budget exhaustion during reduce
returns the best verified witness, false minimality and a stop reason.

Part 3's proposed default line/numeric limits and E-SCALE mapping can change
existing verdicts. Those concrete differences require the R9 review before
enabling them. New opt-in budget flags do not redefine the existing defaults.
