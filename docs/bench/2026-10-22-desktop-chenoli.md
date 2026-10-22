# Jitter benchmark — 2026-10-22

- Machine: 26300.9711 | AMD Ryzen 5 5600X 6-Core Processor | 12 threads
- Power: plugged in
- Timer: highResolution · MMCSS: on
- Duration: 5 s per interval; no-op action (no input injected)
- Reliable means: p99 delay < 25% of the interval and at most 0.1% of the deadlines skipped

## Idle

| Interval | Samples | p50 (µs) | p95 (µs) | p99 (µs) | max (µs) | missed | reliable |
| -------: | ------: | -------: | -------: | -------: | -------: | -----: | :------: |
|     1 ms |    5000 |        1 |        1 |        1 |      136 |      0 |   yes    |
|     2 ms |    2500 |        1 |        1 |        1 |        8 |      0 |   yes    |
|     4 ms |    1250 |        1 |        1 |        1 |      137 |      0 |   yes    |
|     8 ms |     625 |        1 |        1 |        1 |       12 |      0 |   yes    |
|    16 ms |     312 |        1 |        1 |        1 |        0 |      0 |   yes    |

Shortest reliable interval: **1 ms**

## CPU at 100% on every core

| Interval | Samples | p50 (µs) | p95 (µs) | p99 (µs) | max (µs) | missed | reliable |
| -------: | ------: | -------: | -------: | -------: | -------: | -----: | :------: |
|     1 ms |    5000 |        1 |     1024 |     1152 |     2059 |    398 |    no    |
|     2 ms |    2500 |        1 |        1 |        1 |       46 |      0 |   yes    |
|     4 ms |    1250 |        1 |        1 |        6 |       60 |      0 |   yes    |
|     8 ms |     625 |        1 |        1 |        1 |        6 |      0 |   yes    |
|    16 ms |     312 |        1 |        1 |        4 |       40 |      0 |   yes    |

Shortest reliable interval: **2 ms**

## CSV

```csv
scenario,interval_ms,samples,p50_us,p95_us,p99_us,max_us,missed
idle,1,5000,1.0,1.0,1.0,136.3,0
idle,2,2500,1.0,1.0,1.0,7.9,0
idle,4,1250,1.0,1.0,1.0,136.9,0
idle,8,625,1.0,1.0,1.0,12.0,0
idle,16,312,1.0,1.0,1.0,0.0,0
stress,1,5000,1.0,1024.0,1152.0,2059.0,398
stress,2,2500,1.0,1.0,1.0,46.0,0
stress,4,1250,1.0,1.0,6.0,60.2,0
stress,8,625,1.0,1.0,1.0,5.5,0
stress,16,312,1.0,1.0,4.0,40.2,0
```
