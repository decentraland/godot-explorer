## gp

Median of 5 runs (1.x) and 5 runs (2.x), alternating.

| metric | 1.x | 2.x | Δ (2.x vs 1.x) |
|---|---:|---:|---:|
| fps avg | 17.9 | 18.1 | +0.7% (better) |
| fps p1 (1% low) | 14.9 | 13.4 | -10.6% (worse) |
| fps p5 (5% low) | 15.0 | 15.0 | -0.1% (worse) |
| frame time p50 (ms) | 58.3 | 58.2 | -0.0% (better) |
| frame time p95 (ms) | 66.7 | 66.8 | +0.1% (worse) |
| frame time p99 (ms) | 66.9 | 74.9 | +11.9% (worse) |
| frame time max (ms) | 91.6 | 100.1 | +9.3% (worse) |
| RSS steady (MB, PSS) | 2031 | 1967 | -3.2% (better) |
| RSS peak (MB, PSS) | 2094 | 1998 | -4.6% (better) |
| scene draw-ready t100 (ms) | 7710 | 1410 | -81.7% (better) |
| scene draw-ready t90 (ms) | 6965 | 1410 | -79.8% (better) |
| cold launch -> draw-ready (ms) | 27775 | 19433 | -30.0% (better) |
| am start -W TotalTime (ms) | 335 | 316 | -5.7% (better) |
| CPU mean (% of all cores) | 132.7 | 119.0 | -10.3% (better) |

**Budget:** fps ≥ 1.x → **PASS**; RSS ≤ 1.x + 100 MB (2131 MB) → **PASS**

