# Performance results

Rows are added by `scripts/perf.sh` before every release (see [README.md](README.md)). Times in milliseconds; compare rows of the
same machine and of the same test project (its SHA-256).

| Day | Program | Measure | ms | What | Machine | Project |
|-----|---------|---------|----|------|---------|---------|
| 2026-11-27 | LightCreator v1.0.0 (+ preview on the work area) | open | 59 | 944 shapes on face A | Apple M4 Pro | ff181ee7443a |
| 2026-11-27 | LightCreator v1.0.0 (+ preview on the work area) | gcode | 1033233 | 6 faces, 98415 kB | Apple M4 Pro | ff181ee7443a |
| 2026-11-27 | LightCreator v1.0.0 (+ preview on the work area) | preview | 53471 | 6 faces | Apple M4 Pro | ff181ee7443a |
| 2026-11-27 | MillCreator v1.0.0 (+ preview on the work area) | open | 62 | 944 shapes on face A | Apple M4 Pro | ff181ee7443a |
| 2026-11-27 | MillCreator v1.0.0 (+ preview on the work area) | gcode | 236369 | 6 faces, 66012 kB | Apple M4 Pro | ff181ee7443a |
| 2026-11-27 | MillCreator v1.0.0 (+ preview on the work area) | preview | 52504 | 6 faces | Apple M4 Pro | ff181ee7443a |
| 2026-11-27 | PenCreator v1.0.0 (+ preview on the work area) | open | 33 | 944 shapes on face A | Apple M4 Pro | ff181ee7443a |
| 2026-11-27 | PenCreator v1.0.0 (+ preview on the work area) | gcode | 236 | 1 faces, 3327 kB | Apple M4 Pro | ff181ee7443a |
| 2026-11-27 | PenCreator v1.0.0 (+ preview on the work area) | preview | 6 | 1 faces | Apple M4 Pro | ff181ee7443a |
