# Criterion medians

Times are nanoseconds per benchmark iteration. Confidence intervals are stored
median estimates, not cross-machine guarantees. Commands and machine are linked
in COMMANDS.md.

| Workload | Median ns | Confidence interval ns |
|---|---:|---:|
| decimal_parse | 74.097 | 73.143–74.898 |
| mul_rescale_i128 | 11.823 | 11.731–11.983 |
| mul_rescale_wide | 449.476 | 444.201–458.943 |
| pipeline/check_100k | 1833317.934 | 1809658.833–1864298.752 |
| pipeline/parse_100k | 49797303.500 | 49055554.000–50275760.500 |
| pipeline/replay_100k | 369029.066 | 363144.355–375371.347 |
| reduce_100k | 5748523.998 | 5707790.903–5938102.429 |
