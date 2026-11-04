# Criterion medians

Nanoseconds per iteration; 50 samples.

| Workload | Median ns | Median confidence interval ns |
|---|---:|---:|
| decimal_parse | 71.672 | 70.744–73.681 |
| mul_rescale_i128 | 12.325 | 12.138–12.991 |
| mul_rescale_wide | 433.914 | 429.701–440.714 |
| pipeline/check_100k | 2877356.314 | 2823918.377–3011911.408 |
| pipeline/parse_100k | 54987349.000 | 54132601.750–56296653.500 |
| pipeline/replay_100k | 182546.107 | 179034.050–188154.245 |
| reduce_100k | 5603157.761 | 5387513.538–5721575.213 |
