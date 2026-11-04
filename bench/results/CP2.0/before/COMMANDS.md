# CP2.0 before baseline

Source: 281fa154ac3e2cae4a3f2155d025b91c57b30e97.

```sh
cargo nextest run --workspace --locked
RAYON_NUM_THREADS=5 cargo bench -p qlrs-bench --bench throughput --locked -- pipeline/check_100k --sample-size 50
cargo test -p qlrs-testkit --test part2 --locked
```

The performance workload uses the unchanged pre-CP2.0 crate sources. The new
regression file is not part of that benchmark's build. Stored estimates and 50
samples are copied without modification. The regression run is intentionally
failing: it reproduces verdict precedence and unrelated I2 witness evidence.
The D04 size/budget regression already passes; existing Stage A is reused.
Machine/OS/quota/compiler and official stable-channel metadata are retained.
