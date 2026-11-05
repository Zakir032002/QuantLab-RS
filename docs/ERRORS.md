# Input and resource errors

The first stderr line contains the identifier. Typed ingestion failures also
produce JSON with `status`, `code`, `line` (one-based; zero for file operations)
and `message`. Resource flags currently opt into the proposed Part 3 policy;
see ADR-0017 and the pending default-verdict review.

| Identifier | Meaning | Exit |
|---|---|---|
| E-JSON | Invalid JSON, record or decimal syntax | 1 |
| E-DUPLICATE-KEY | Duplicate decoded object key | 1 |
| E-UNKNOWN-KEY | Unknown schema field | 1 |
| E-UTF8 | Invalid UTF-8 or Unicode escape | 1 |
| E-NEWLINE | Missing final line terminator | 1 |
| E-IO | File operation failed | 1 |
| E-LIMIT-CONFIG | Configured limit exceeds its hard ceiling | 1 |
| E-LIMIT-LINE-BYTES | Line exceeds its byte limit, before buffering whole | 1 |
| E-LIMIT-DEPTH | JSON nesting exceeds its limit | 1 |
| E-LIMIT-NUMBER-LEN | Numeric token or decimal string exceeds its length limit | 1 |
| E-SCALE | Decimal exceeds the explicitly selected scale limit | 1 |
| E-LIMIT-STRING | Decoded string exceeds its byte limit | 1 |
| E-LIMIT-INPUT-BYTES | Input exceeds its byte limit | 1 |
| E-INPUT-NOT-SEEKABLE | Input cannot be used without spooling | 1 |
| U-LIMIT-INTERNED | Interner capacity exceeded | 4 |
| U-LIMIT-EVENTS | Event capacity exceeded | 4 |
| U-LIMIT-INDEX | Reserved or unrepresentable column/CSR index | 4 |
| I-BUDGET-MEMORY | User allocation budget exceeded | 3 |
| E-BUNDLE-PATH | Invalid, duplicate, symlink or unexpected bundle path | 1 |
| E-BUNDLE-ENTRIES | Bundle entry count exceeded | 1 |
| E-BUNDLE-ENTRY-BYTES | Bundle entry byte limit exceeded | 1 |

Existing arithmetic overflow and model findings retain their established SPEC
verdicts. A resource error never truncates a trace into a successful result.
