# Criterion medians

Nanoseconds per iteration; 50 samples.

| Workload | Median ns | Median confidence interval ns |
|---|---:|---:|
| decimal_parse | 97.433 | 92.526–105.583 |
| mul_rescale_i128 | 6.549 | 6.265–7.119 |
| mul_rescale_wide | 797.981 | 747.713–848.804 |
| pipeline/check_100k | 2317846.383 | 2259968.403–2399087.439 |
| pipeline/parse_100k | 67010916.750 | 54013583.000–79229364.750 |
| pipeline/replay_100k | 197929.436 | 185910.025–203654.659 |
| reduce_100k | 6711402.135 | 6007278.688–7195781.250 |
