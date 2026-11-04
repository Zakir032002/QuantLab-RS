# Criterion medians

Nanoseconds per iteration; 50 samples.

| Workload | Median ns | Median confidence interval ns |
|---|---:|---:|
| decimal_parse | 104.818 | 94.640–111.839 |
| mul_rescale_i128 | 6.211 | 5.703–6.464 |
| mul_rescale_wide | 656.373 | 644.288–710.496 |
| pipeline/check_100k | 1874445.896 | 1713049.413–2010221.154 |
| pipeline/parse_100k | 65270448.000 | 61961958.500–74034906.250 |
| pipeline/replay_100k | 296720.012 | 286131.949–306336.438 |
| reduce_100k | 5192979.674 | 4907124.389–5429119.812 |
