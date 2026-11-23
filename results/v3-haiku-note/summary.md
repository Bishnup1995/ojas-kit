# Benchmark v2 summary (120 valid runs, 0 invalid, 0 incomplete) from ..\results\v3-haiku-note\raw

## Block: stress  (40 runs, tasks: s1_click_full_suite, s2_express_full_suite, s3_click_core_full_read, s4_log_cat_all)

| arm | n | pass | mean cost $ | cost/correct $ | turns | input | cache write | cache read | output | wall s | caps | timeouts |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| baseline | 20 | 15/20 | 0.0055 | 0.0073 | 2.9 | 6 | 20717 | 53314 | 1644 | 14 | 0 | 0 |
| ojas3 | 20 | 19/20 | 0.0026 | 0.0027 | 2.5 | 5 | 6849 | 48757 | 1402 | 12 | 0 | 0 |

Ratio vs baseline, task-balanced means, 95% stratified bootstrap interval; pass rate Fisher exact p:

| arm | cost | turns | cache read | cache write | output | pass rate (p) |
|---|---|---|---|---|---|---|
| ojas3 | 0.47x [0.42, 0.51] | 0.86x [0.78, 0.95] | 0.91x [0.80, 1.05] | 0.33x [0.27, 0.38] | 0.85x [0.74, 0.97] | 19/20 vs 15/20 (p=0.182) |

Per task: mean cost $ / mean turns (passes/n)

| task | baseline | ojas3 | headroom | rtk |
|---|---|---|---|---|
| s1_click_full_suite | 0.0020 / 2.2 (5/5) | 0.0023 / 2.4 (5/5) | - | - |
| s2_express_full_suite | 0.0019 / 2.0 (0/5) | 0.0013 / 2.0 (5/5) | - | - |
| s3_click_core_full_read | 0.0149 / 3.0 (5/5) | 0.0040 / 2.8 (4/5) | - | - |
| s4_log_cat_all | 0.0031 / 4.2 (5/5) | 0.0027 / 2.6 (5/5) | - | - |

Tool activity: ojas3: ojas_stats_entries total=21 (runs>0: 20/20), ojas_read_tool_outlines total=0 (runs>0: 0/20), ojas_grep_tool total=0 (runs>0: 0/20)

## Block: main  (80 runs, tasks: t1_click_bug, t2_express_bug, t3_walkdir_bug, t4_click_rename, t5_click_qa, t6_log_rootcause, t7_git_history, t8_control_edit)

| arm | n | pass | mean cost $ | cost/correct $ | turns | input | cache write | cache read | output | wall s | caps | timeouts |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| baseline | 40 | 40/40 | 0.0030 | 0.0030 | 5.7 | 10 | 6824 | 95977 | 1272 | 19 | 0 | 0 |
| ojas3 | 40 | 40/40 | 0.0033 | 0.0033 | 6.0 | 11 | 7283 | 112644 | 1487 | 21 | 0 | 0 |

Ratio vs baseline, task-balanced means, 95% stratified bootstrap interval; pass rate Fisher exact p:

| arm | cost | turns | cache read | cache write | output | pass rate (p) |
|---|---|---|---|---|---|---|
| ojas3 | 1.12x [1.05, 1.21] | 1.07x [0.99, 1.15] | 1.17x [1.10, 1.26] | 1.07x [0.96, 1.19] | 1.17x [1.07, 1.27] | 40/40 vs 40/40 (p=1.000) |

Per task: mean cost $ / mean turns (passes/n)

| task | baseline | ojas3 | headroom | rtk |
|---|---|---|---|---|
| t1_click_bug | 0.0037 / 6.6 (5/5) | 0.0040 / 7.0 (5/5) | - | - |
| t2_express_bug | 0.0037 / 7.2 (5/5) | 0.0045 / 8.4 (5/5) | - | - |
| t3_walkdir_bug | 0.0064 / 10.2 (5/5) | 0.0076 / 11.0 (5/5) | - | - |
| t4_click_rename | 0.0022 / 5.0 (5/5) | 0.0028 / 6.8 (5/5) | - | - |
| t5_click_qa | 0.0018 / 5.4 (5/5) | 0.0019 / 5.4 (5/5) | - | - |
| t6_log_rootcause | 0.0032 / 5.2 (5/5) | 0.0030 / 4.4 (5/5) | - | - |
| t7_git_history | 0.0014 / 2.0 (5/5) | 0.0015 / 2.0 (5/5) | - | - |
| t8_control_edit | 0.0013 / 3.8 (5/5) | 0.0012 / 3.4 (5/5) | - | - |

Tool activity: ojas3: ojas_stats_entries total=36 (runs>0: 21/40), ojas_read_tool_outlines total=0 (runs>0: 0/40), ojas_grep_tool total=0 (runs>0: 0/40)

Invalid runs (excluded): none
Incomplete dirs (in progress or failed infra): none

