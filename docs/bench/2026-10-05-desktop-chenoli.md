# Jitter benchmark — 2026-10-05

- Machine: 26300.9550 | AMD Ryzen 5 5600X 6-Core Processor | 12 threads
- Power: plugged in
- Timer: highResolution · MMCSS: on
- Duration: 5 s per interval; no-op action (no input injected)
- Reliable means: p99 delay < 25% of the interval and no missed events

## Idle

| Interval | Samples | p50 (µs) | p95 (µs) | p99 (µs) | max (µs) | missed | reliable |
| -------: | ------: | -------: | -------: | -------: | -------: | -----: | :------: |
|     1 ms |    5000 |        1 |        1 |        1 |     1640 |      0 |   yes    |
|     2 ms |    2500 |        1 |        1 |        1 |      128 |      0 |   yes    |
|     4 ms |    1250 |        1 |        1 |        1 |        0 |      0 |   yes    |
|     8 ms |     625 |        1 |        1 |        1 |       14 |      0 |   yes    |
|    16 ms |     312 |        1 |        1 |        1 |      188 |      0 |   yes    |

Shortest reliable interval: **1 ms**

## CPU at 100% on every core

| Interval | Samples | p50 (µs) | p95 (µs) | p99 (µs) | max (µs) | missed | reliable |
| -------: | ------: | -------: | -------: | -------: | -------: | -----: | :------: |
|     1 ms |    5000 |        1 |     1408 |     1920 |     2035 |      0 |    no    |
|     2 ms |    2500 |        1 |        1 |        1 |      440 |      0 |   yes    |
|     4 ms |    1250 |        1 |        1 |        1 |       11 |      0 |   yes    |
|     8 ms |     625 |        1 |        1 |        2 |       42 |      0 |   yes    |
|    16 ms |     312 |        1 |        1 |        1 |        3 |      0 |   yes    |

Shortest reliable interval: **2 ms**

## CSV

```csv
scenario,interval_ms,samples,p50_us,p95_us,p99_us,max_us,missed
idle,1,5000,1.0,1.0,1.0,1639.9,0
idle,2,2500,1.0,1.0,1.0,127.6,0
idle,4,1250,1.0,1.0,1.0,0.4,0
idle,8,625,1.0,1.0,1.0,13.8,0
idle,16,312,1.0,1.0,1.0,187.5,0
stress,1,5000,1.0,1408.0,1920.0,2035.2,0
stress,2,2500,1.0,1.0,1.0,440.2,0
stress,4,1250,1.0,1.0,1.0,10.6,0
stress,8,625,1.0,1.0,2.0,42.1,0
stress,16,312,1.0,1.0,1.0,2.7,0
```
