# Benchmark v2 summary (10 valid runs, 0 invalid, 0 incomplete) from raw

## Block: long  (10 runs, tasks: l6_oncall_day)

| arm | n | pass | mean cost $ | cost/correct $ | turns | input | cache write | cache read | output | wall s | caps | timeouts |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| baseline | 5 | 5/5 | 0.6689 | 0.6689 | 48.8 | 96 | 62941 | 2384788 | 17845 | 329 | 0 | 0 |
| ojas3 | 5 | 5/5 | 0.7140 | 0.7140 | 53.8 | 107 | 61399 | 2746804 | 19355 | 326 | 0 | 0 |

Ratio vs baseline, task-balanced means, 95% stratified bootstrap interval; pass rate Fisher exact p:

| arm | cost | turns | cache read | cache write | output | pass rate (p) |
|---|---|---|---|---|---|---|
| ojas3 | 1.07x [0.88, 1.30] | 1.10x [0.97, 1.24] | 1.15x [0.90, 1.45] | 0.98x [0.82, 1.14] | 1.08x [0.92, 1.30] | 5/5 vs 5/5 (p=1.000) |

Per task: mean cost $ / mean turns (passes/n)

| task | baseline | ojas3 | headroom | rtk |
|---|---|---|---|---|
| l6_oncall_day | 0.6689 / 48.8 (5/5) | 0.7140 / 53.8 (5/5) | - | - |

Tool activity: ojas3: ojas_stats_entries total=42 (runs>0: 5/5), ojas_read_tool_outlines total=0 (runs>0: 0/5), ojas_grep_tool total=0 (runs>0: 0/5)

Invalid runs (excluded): none
Incomplete dirs (in progress or failed infra): none

## Notes (written after analysis)

- Validity check passed: median turns 50 (no tool) and 54 (ojas), both >= 25. These are real long sessions.
- H4c (ojas cheaper in long sessions): not supported. Cost 1.07x [0.88, 1.30], interval includes 1.0.
- H5c (pass rate not lower): held, 5/5 vs 5/5.
- Decision rule: no fix triggered (interval includes 1.0, no lost runs).
- No ojas run called ojas get or OJAS_RAW=1, so the extra turns (1.10x [0.97, 1.24]) did not come from the agent recovering cut output; ojas run 5 alone took 66 turns.
- Repeated test runs: median 25.0% (pooled 24.5%) of tool-output tokens in the no-tool runs, exactly at the pre-registered 25% bar. Borderline: weak support for building the rerun diff.
