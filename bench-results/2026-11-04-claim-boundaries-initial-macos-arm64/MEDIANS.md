# Criterion medians

Nanoseconds per iteration; 50 samples.

| Workload | Median ns | Median confidence interval ns |
|---|---:|---:|
| decimal_parse | 83.684 | 82.059–84.701 |
| mul_rescale_i128 | 5.677 | 5.625–5.984 |
| mul_rescale_wide | 623.821 | 594.498–654.142 |
| pipeline/check_100k | 2162752.850 | 2111502.964–2259565.000 |
| pipeline/parse_100k | 73582552.250 | 55331260.500–82822656.500 |
| pipeline/replay_100k | 174516.175 | 173644.705–176306.897 |
| reduce_100k | 5873190.236 | 5660246.769–6022578.797 |
