# Criterion medians

Nanoseconds per iteration; 50 samples.

| Workload | Median ns | Median confidence interval ns |
|---|---:|---:|
| decimal_parse | 68.113 | 67.694–69.110 |
| mul_rescale_i128 | 11.983 | 11.914–12.113 |
| mul_rescale_wide | 469.525 | 453.201–489.050 |
| pipeline/parse_100k | 68315567.000 | 66622144.000–72485577.000 |
| pipeline/replay_100k | 179124.042 | 174233.666–184858.504 |
| pipeline/check_100k | 2766142.681 | 2725885.976–2818155.389 |
| reduce_100k | 5454098.748 | 5376586.936–5588955.357 |
