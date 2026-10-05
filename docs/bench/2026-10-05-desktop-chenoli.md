# Benchmark de jitter — 2026-10-05

- Máquina: 26300.9550 | AMD Ryzen 5 5600X 6-Core Processor | 12 threads
- Energia: tomada
- Timer: highResolution · MMCSS: ativo
- Duração: 5 s por intervalo; ação no-op (nenhum input injetado)
- Critério de confiável: p99 do atraso < 25% do intervalo e nenhum evento perdido

## Ocioso

| Intervalo | Amostras | p50 (µs) | p95 (µs) | p99 (µs) | máx (µs) | perdidos | confiável |
| --------: | -------: | -------: | -------: | -------: | -------: | -------: | :-------: |
|      1 ms |     5000 |        1 |        1 |        1 |     1640 |        0 |    sim    |
|      2 ms |     2500 |        1 |        1 |        1 |      128 |        0 |    sim    |
|      4 ms |     1250 |        1 |        1 |        1 |        0 |        0 |    sim    |
|      8 ms |      625 |        1 |        1 |        1 |       14 |        0 |    sim    |
|     16 ms |      312 |        1 |        1 |        1 |      188 |        0 |    sim    |

Menor intervalo confiável: **1 ms**

## CPU 100% em todos os núcleos

| Intervalo | Amostras | p50 (µs) | p95 (µs) | p99 (µs) | máx (µs) | perdidos | confiável |
| --------: | -------: | -------: | -------: | -------: | -------: | -------: | :-------: |
|      1 ms |     5000 |        1 |     1408 |     1920 |     2035 |        0 |    não    |
|      2 ms |     2500 |        1 |        1 |        1 |      440 |        0 |    sim    |
|      4 ms |     1250 |        1 |        1 |        1 |       11 |        0 |    sim    |
|      8 ms |      625 |        1 |        1 |        2 |       42 |        0 |    sim    |
|     16 ms |      312 |        1 |        1 |        1 |        3 |        0 |    sim    |

Menor intervalo confiável: **2 ms**

## CSV

```csv
cenario,intervalo_ms,amostras,p50_us,p95_us,p99_us,max_us,perdidos
ocioso,1,5000,1.0,1.0,1.0,1639.9,0
ocioso,2,2500,1.0,1.0,1.0,127.6,0
ocioso,4,1250,1.0,1.0,1.0,0.4,0
ocioso,8,625,1.0,1.0,1.0,13.8,0
ocioso,16,312,1.0,1.0,1.0,187.5,0
stress,1,5000,1.0,1408.0,1920.0,2035.2,0
stress,2,2500,1.0,1.0,1.0,440.2,0
stress,4,1250,1.0,1.0,1.0,10.6,0
stress,8,625,1.0,1.0,2.0,42.1,0
stress,16,312,1.0,1.0,1.0,2.7,0
```
