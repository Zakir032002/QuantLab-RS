# Criterion medians

Nanoseconds per iteration; 50 samples.

| Workload | Median ns | Median confidence interval ns |
|---|---:|---:|
| decimal_parse | 121.699 | 115.982–125.143 |
| mul_rescale_i128 | 21.359 | 20.673–21.850 |
| mul_rescale_wide | 697.245 | 664.658–735.521 |
| pipeline/check_100k | 8966491.750 | 8528943.167–9353668.750 |
| pipeline/parse_100k | 162311312.500 | 147441977.000–197634171.000 |
| pipeline/replay_100k | 317094.912 | 272312.427–338033.242 |
| reduce_100k | 19655516.100 | 19147304.700–20381592.400 |
