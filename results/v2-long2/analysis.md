# Benchmark v2 summary (20 valid runs, 0 invalid, 0 incomplete) from results\v2-long2\raw

## Block: long  (20 runs, tasks: l4_click_sprint, l5_express_ci_plus)

| arm | n | pass | mean cost $ | cost/correct $ | turns | input | cache write | cache read | output | wall s | caps | timeouts |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| baseline | 10 | 10/10 | 0.2357 | 0.2357 | 18.2 | 36 | 27457 | 546475 | 7117 | 112 | 0 | 0 |
| ojas3 | 10 | 10/10 | 0.2403 | 0.2403 | 19.1 | 38 | 26736 | 585617 | 7471 | 136 | 0 | 0 |

Ratio vs baseline, task-balanced means, 95% stratified bootstrap interval; pass rate Fisher exact p:

| arm | cost | turns | cache read | cache write | output | pass rate (p) |
|---|---|---|---|---|---|---|
| ojas3 | 1.02x [0.94, 1.10] | 1.05x [0.99, 1.11] | 1.07x [0.98, 1.17] | 0.97x [0.90, 1.05] | 1.05x [0.93, 1.18] | 10/10 vs 10/10 (p=1.000) |

Per task: mean cost $ / mean turns (passes/n)

| task | baseline | ojas3 | headroom | rtk |
|---|---|---|---|---|
| l4_click_sprint | 0.3632 / 24.4 (5/5) | 0.3768 / 26.0 (5/5) | - | - |
| l5_express_ci_plus | 0.1082 / 12.0 (5/5) | 0.1038 / 12.2 (5/5) | - | - |

Tool activity: ojas3: ojas_stats_entries total=23 (runs>0: 8/10), ojas_read_tool_outlines total=0 (runs>0: 0/10), ojas_grep_tool total=0 (runs>0: 0/10)

Invalid runs (excluded): none
Incomplete dirs (in progress or failed infra): none

## Notes (written after analysis)

- Result-event merge: in ojas3 l5 runs 3 and 4 a background command the agent started (its own /tmp/t.js repro server) finished after the task, so Claude Code emitted a second 1-turn `result` event. num_turns, usage and duration_ms are per event and total_cost_usd is cumulative; analyze_v2.py now sums the per-event fields over all result events. Turns become 13 and 14. Before the merge, turns read 0.91x [0.78, 1.04] and cache write 0.87x [0.75, 0.99]; after it, no token measure differs clearly.
- The same two runs show 741 s wall time: about 120 s for the hung command plus about 575 s of Claude Code waiting for the background task at session end. ojas did not rewrite that command (`ojas rewrite` returns it unchanged). Baseline runs 1, 4 and 5 wrote similar repro servers.
- Validity check (median turns >= 25 for both arms): l4 baseline 24, ojas 26 -> fails narrowly. l5 baseline 12, ojas 13 -> fails. Per LONG2_PREREG.md, H4b is reported as "not long enough".
- Decision rule: cost 1.02x [0.94, 1.10] includes 1.0 and pass rate 10/10 vs 10/10, so no fix is triggered.
