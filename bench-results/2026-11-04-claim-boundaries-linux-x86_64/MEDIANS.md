# Criterion medians

Nanoseconds per iteration; 50 samples.

| Workload | Median ns | Median confidence interval ns |
|---|---:|---:|
| decimal_parse | 70.396 | 69.096–72.919 |
| mul_rescale_i128 | 12.393 | 12.246–12.611 |
| mul_rescale_wide | 427.209 | 423.491–431.156 |
| pipeline/parse_100k | 55767928.000 | 53909910.250–57105512.000 |
| pipeline/replay_100k | 170648.748 | 168786.816–175129.430 |
| pipeline/check_100k | 2789877.187 | 2754227.870–2828904.982 |
| reduce_100k | 5571154.247 | 5486949.690–5686910.567 |
