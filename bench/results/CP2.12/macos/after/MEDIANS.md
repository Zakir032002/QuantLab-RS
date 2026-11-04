# Criterion medians

Nanoseconds per iteration; 50 samples.

| Workload | Median ns | Median confidence interval ns |
|---|---:|---:|
| decimal_parse | 107.803 | 106.138–112.384 |
| mul_rescale_i128 | 15.665 | 15.625–15.754 |
| mul_rescale_wide | 472.120 | 468.383–476.986 |
| pipeline/check_100k | 4648643.652 | 4631073.984–4675085.413 |
| pipeline/parse_100k | 105316599.000 | 103592093.500–106869874.000 |
| pipeline/replay_100k | 346852.331 | 344546.568–350371.785 |
| reduce_100k | 11763659.125 | 11589471.312–11856608.312 |
