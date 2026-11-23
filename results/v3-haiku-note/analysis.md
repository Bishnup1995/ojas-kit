# Haiku confirmation of the session-note fix (exploratory; see v2/DEVIATIONS.md, 2026-11-23)

Same schedule, model (claude-haiku-5-5), tasks and baseline image as v3-haiku; ojas arm built from ojas-kit af94fe5 (session note no longer mentions OJAS_RAW=1). 120/120 runs valid, no rate-limit pauses. Full tables: summary.md.

| | v3-haiku (old note) | v3-haiku-note (new note) |
|---|---|---|
| ojas runs with an `OJAS_RAW=1` command | 18/60 | 0/60 |
| Big output: cost vs baseline | 1.07x [0.98, 1.16] | 0.47x [0.42, 0.51] |
| Big output: passed (ojas vs baseline) | 16/20 vs 15/20 | 19/20 vs 15/20 (p=0.18) |
| s2 express failing test | 1/5 vs 0/5 | 5/5 vs 0/5 |
| Ordinary tasks: cost vs baseline | 1.17x [1.05, 1.30] | 1.12x [1.05, 1.21] |
| Ordinary tasks: passed | 40/40 vs 40/40 | 40/40 vs 40/40 |

Findings
- The bypass is gone, and on big output ojas now helps on Haiku as on Sonnet (Sonnet v2: 0.88x, 40/40).
- s3 (one full read of click's 4,018-line core.py): cost 0.0149 -> 0.0040, but run #5 failed: `cat` returned an outline, the task forbade any other read, and the agent said it could not answer. In v3-haiku the ojas runs passed s3 by using OJAS_RAW=1. Trade-off of the note change: with no advertised raw escape, "read the whole file in one go" fails when the file is outlined.
- Ordinary tasks still cost more on Haiku (1.12x [1.05, 1.21]), from extra turns on t2, t3 and t4, not from the bypass. Sonnet showed no overhead (0.97x). Unexplained; not a pass-rate problem.
