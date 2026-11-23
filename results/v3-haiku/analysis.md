# Benchmark v2 summary (120 valid runs, 0 invalid, 0 incomplete) from raw

## Block: stress  (40 runs, tasks: s1_click_full_suite, s2_express_full_suite, s3_click_core_full_read, s4_log_cat_all)

| arm | n | pass | mean cost $ | cost/correct $ | turns | input | cache write | cache read | output | wall s | caps | timeouts |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| baseline | 20 | 15/20 | 0.0056 | 0.0074 | 2.9 | 6 | 20912 | 53991 | 1689 | 15 | 0 | 0 |
| ojas3 | 20 | 16/20 | 0.0060 | 0.0075 | 2.7 | 5 | 21767 | 54380 | 2142 | 16 | 0 | 0 |

Ratio vs baseline, task-balanced means, 95% stratified bootstrap interval; pass rate Fisher exact p:

| arm | cost | turns | cache read | cache write | output | pass rate (p) |
|---|---|---|---|---|---|---|
| ojas3 | 1.07x [0.98, 1.16] | 0.93x [0.86, 1.02] | 1.01x [0.92, 1.10] | 1.04x [0.95, 1.14] | 1.27x [1.12, 1.43] | 16/20 vs 15/20 (p=1.000) |

Per task: mean cost $ / mean turns (passes/n)

| task | baseline | ojas3 | headroom | rtk |
|---|---|---|---|---|
| s1_click_full_suite | 0.0022 / 2.4 (5/5) | 0.0025 / 2.2 (5/5) | - | - |
| s2_express_full_suite | 0.0019 / 2.0 (0/5) | 0.0020 / 2.0 (1/5) | - | - |
| s3_click_core_full_read | 0.0150 / 3.0 (5/5) | 0.0159 / 3.2 (5/5) | - | - |
| s4_log_cat_all | 0.0031 / 4.2 (5/5) | 0.0034 / 3.4 (5/5) | - | - |

Tool activity: ojas3: ojas_stats_entries total=10 (runs>0: 9/20), ojas_read_tool_outlines total=0 (runs>0: 0/20), ojas_grep_tool total=0 (runs>0: 0/20)

## Block: main  (80 runs, tasks: t1_click_bug, t2_express_bug, t3_walkdir_bug, t4_click_rename, t5_click_qa, t6_log_rootcause, t7_git_history, t8_control_edit)

| arm | n | pass | mean cost $ | cost/correct $ | turns | input | cache write | cache read | output | wall s | caps | timeouts |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| baseline | 40 | 40/40 | 0.0030 | 0.0030 | 5.6 | 10 | 6778 | 95195 | 1285 | 22 | 0 | 0 |
| ojas3 | 40 | 40/40 | 0.0034 | 0.0034 | 6.0 | 11 | 7513 | 114889 | 1572 | 24 | 0 | 0 |

Ratio vs baseline, task-balanced means, 95% stratified bootstrap interval; pass rate Fisher exact p:

| arm | cost | turns | cache read | cache write | output | pass rate (p) |
|---|---|---|---|---|---|---|
| ojas3 | 1.17x [1.05, 1.30] | 1.07x [0.98, 1.18] | 1.21x [1.06, 1.38] | 1.11x [1.00, 1.23] | 1.22x [1.06, 1.42] | 40/40 vs 40/40 (p=1.000) |

Per task: mean cost $ / mean turns (passes/n)

| task | baseline | ojas3 | headroom | rtk |
|---|---|---|---|---|
| t1_click_bug | 0.0034 / 6.0 (5/5) | 0.0036 / 6.2 (5/5) | - | - |
| t2_express_bug | 0.0043 / 8.4 (5/5) | 0.0049 / 9.2 (5/5) | - | - |
| t3_walkdir_bug | 0.0057 / 9.6 (5/5) | 0.0080 / 11.0 (5/5) | - | - |
| t4_click_rename | 0.0022 / 5.0 (5/5) | 0.0027 / 5.8 (5/5) | - | - |
| t5_click_qa | 0.0018 / 5.0 (5/5) | 0.0017 / 4.4 (5/5) | - | - |
| t6_log_rootcause | 0.0035 / 5.0 (5/5) | 0.0039 / 6.2 (5/5) | - | - |
| t7_git_history | 0.0015 / 2.2 (5/5) | 0.0016 / 2.0 (5/5) | - | - |
| t8_control_edit | 0.0012 / 3.8 (5/5) | 0.0012 / 3.4 (5/5) | - | - |

Tool activity: ojas3: ojas_stats_entries total=48 (runs>0: 24/40), ojas_read_tool_outlines total=0 (runs>0: 0/40), ojas_grep_tool total=0 (runs>0: 0/40)

Invalid runs (excluded): none
Incomplete dirs (in progress or failed infra): none

## Notes (written after analysis)

- H2h (ojas cheaper on big-output tasks, Haiku): not supported. 1.07x [0.98, 1.16].
- H3h (ordinary tasks within 10%): not supported. 1.17x [1.05, 1.30]; turns 1.07x [0.98, 1.18].
- H5h (pass rate not lower): held. Big output 16/20 vs 15/20; ordinary 40/40 vs 40/40.
- Cause found on s2 (find the failing test): in 4 of 5 ojas runs Haiku prefixed its first command with OJAS_RAW=1, which the ojas session note advertises as the way to skip compression. The raw output then hit Claude Code's truncation exactly as with no tool, and the run failed. The one run without OJAS_RAW passed (1991 lines -> 131).
- Haiku used OJAS_RAW=1 in 18 of 60 ojas runs (29 commands); Sonnet in v2 in 16 of 180; on-call day 0 of 5.
- Ordinary tasks: most Haiku commands pipe into tail/grep/head, which ojas leaves alone, so ojas rarely acted; the cost gap is mostly more turns and OJAS_RAW re-runs.
- Action: reword the session note so it doesn't invite the bypass (ojas fix on v0.4-dev), then a small confirmation arm (deviation).
