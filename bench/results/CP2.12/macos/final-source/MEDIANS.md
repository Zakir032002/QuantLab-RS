# Criterion medians

Nanoseconds per iteration; 50 samples.

| Workload | Median ns | Median confidence interval ns |
|---|---:|---:|
| decimal_parse | 94.289 | 85.435–106.499 |
| mul_rescale_i128 | 17.473 | 14.736–21.051 |
| mul_rescale_wide | 580.211 | 545.921–608.242 |
| pipeline/check_100k | 4376066.600 | 4319741.299–4539606.372 |
| pipeline/parse_100k | 91885584.500 | 88992006.500–94590337.000 |
| pipeline/replay_100k | 258342.960 | 237887.398–315281.859 |
| reduce_100k | 11905857.000 | 10673112.875–12736861.125 |
