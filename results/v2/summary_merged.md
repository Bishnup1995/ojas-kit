<!-- Recomputed 2026-11-23 with merged result events (see v2/DEVIATIONS.md). Costs and pass rates are identical to summary.md; only turns, tokens and wall in the main block change (7 t2 runs). -->
# Benchmark v2 summary (720 valid runs, 0 invalid, 0 incomplete) from results\v2\raw

## Block: read  (120 runs, tasks: r1_pricing_policy, r2_ledger_raises, r3_click_core_bug)

| arm | n | pass | mean cost $ | cost/correct $ | turns | input | cache write | cache read | output | wall s | caps | timeouts |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| baseline | 30 | 30/30 | 0.0615 | 0.0615 | 5.3 | 10 | 7510 | 146048 | 1685 | 25 | 0 | 0 |
| ojas3 | 30 | 29/30 | 0.0667 | 0.0690 | 5.2 | 9 | 9885 | 97547 | 1742 | 25 | 0 | 0 |
| headroom | 30 | 30/30 | 0.0707 | 0.0707 | 5.2 | 10 | 9922 | 149592 | 1604 | 43 | 0 | 0 |
| rtk | 30 | 30/30 | 0.0582 | 0.0582 | 5.6 | 10 | 6265 | 154850 | 1763 | 23 | 0 | 0 |

Ratio vs baseline, task-balanced means, 95% stratified bootstrap interval; pass rate Fisher exact p:

| arm | cost | turns | cache read | cache write | output | pass rate (p) |
|---|---|---|---|---|---|---|
| ojas3 | 1.08x [0.85, 1.36] | 0.99x [0.93, 1.07] | 0.67x [0.60, 0.74] | 1.32x [0.81, 2.10] | 1.03x [0.93, 1.15] | 29/30 vs 30/30 (p=1.000) |
| headroom | 1.15x [0.88, 1.49] | 0.99x [0.93, 1.07] | 1.02x [0.91, 1.16] | 1.32x [0.78, 2.21] | 0.95x [0.87, 1.04] | 30/30 vs 30/30 (p=1.000) |
| rtk | 0.95x [0.76, 1.17] | 1.06x [0.99, 1.12] | 1.06x [0.99, 1.14] | 0.83x [0.52, 1.34] | 1.05x [0.98, 1.12] | 30/30 vs 30/30 (p=1.000) |

Per task: mean cost $ / mean turns (passes/n)

| task | baseline | ojas3 | headroom | rtk |
|---|---|---|---|---|
| r1_pricing_policy | 0.0426 / 6.3 (10/10) | 0.0207 / 5.9 (10/10) | 0.0505 / 6.1 (10/10) | 0.0372 / 6.2 (10/10) |
| r2_ledger_raises | 0.0885 / 3.7 (10/10) | 0.1310 / 4.6 (9/10) | 0.1113 / 4.3 (10/10) | 0.0800 / 4.0 (10/10) |
| r3_click_core_bug | 0.0534 / 5.8 (10/10) | 0.0485 / 5.2 (10/10) | 0.0504 / 5.3 (10/10) | 0.0573 / 6.5 (10/10) |

Tool activity: ojas3: ojas_stats_entries total=25 (runs>0: 21/30), ojas_read_tool_outlines total=21 (runs>0: 20/30), ojas_grep_tool total=0 (runs>0: 0/30); rtk: rtk_commands total=79 (runs>0: 29/30)

## Block: stress  (160 runs, tasks: s1_click_full_suite, s2_express_full_suite, s3_click_core_full_read, s4_log_cat_all)

| arm | n | pass | mean cost $ | cost/correct $ | turns | input | cache write | cache read | output | wall s | caps | timeouts |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| baseline | 40 | 28/40 | 0.1018 | 0.1454 | 3.0 | 6 | 21647 | 54687 | 974 | 12 | 0 | 0 |
| ojas3 | 40 | 40/40 | 0.0898 | 0.0898 | 2.5 | 5 | 19432 | 51552 | 688 | 9 | 0 | 0 |
| headroom | 40 | 34/40 | 0.1012 | 0.1190 | 3.3 | 7 | 21340 | 59079 | 991 | 21 | 0 | 0 |
| rtk | 40 | 26/40 | 0.1026 | 0.1579 | 3.9 | 8 | 21034 | 71564 | 1130 | 15 | 0 | 0 |

Ratio vs baseline, task-balanced means, 95% stratified bootstrap interval; pass rate Fisher exact p:

| arm | cost | turns | cache read | cache write | output | pass rate (p) |
|---|---|---|---|---|---|---|
| ojas3 | 0.88x [0.79, 0.97] | 0.83x [0.77, 0.90] | 0.94x [0.84, 1.06] | 0.90x [0.80, 0.99] | 0.71x [0.63, 0.79] | 40/40 vs 28/40 (p=0.000) |
| headroom | 0.99x [0.89, 1.10] | 1.09x [0.98, 1.22] | 1.08x [0.92, 1.27] | 0.99x [0.87, 1.10] | 1.02x [0.91, 1.14] | 34/40 vs 28/40 (p=0.180) |
| rtk | 1.01x [0.88, 1.14] | 1.29x [1.15, 1.46] | 1.31x [1.13, 1.51] | 0.97x [0.83, 1.11] | 1.16x [1.04, 1.31] | 26/40 vs 28/40 (p=0.812) |

Per task: mean cost $ / mean turns (passes/n)

| task | baseline | ojas3 | headroom | rtk |
|---|---|---|---|---|
| s1_click_full_suite | 0.0267 / 2.3 (10/10) | 0.0293 / 2.0 (10/10) | 0.0334 / 2.4 (10/10) | 0.0400 / 5.3 (10/10) |
| s2_express_full_suite | 0.0277 / 2.0 (0/10) | 0.0152 / 2.0 (10/10) | 0.0339 / 2.8 (4/10) | 0.0492 / 3.0 (0/10) |
| s3_click_core_full_read | 0.2799 / 3.1 (10/10) | 0.3030 / 4.0 (10/10) | 0.2797 / 3.2 (10/10) | 0.2635 / 3.0 (10/10) |
| s4_log_cat_all | 0.0731 / 4.6 (8/10) | 0.0116 / 2.0 (10/10) | 0.0578 / 4.7 (10/10) | 0.0577 / 4.2 (6/10) |

Tool activity: ojas3: ojas_stats_entries total=40 (runs>0: 40/40), ojas_read_tool_outlines total=0 (runs>0: 0/40), ojas_grep_tool total=0 (runs>0: 0/40); rtk: rtk_commands total=23 (runs>0: 17/40)

## Block: main  (320 runs, tasks: t1_click_bug, t2_express_bug, t3_walkdir_bug, t4_click_rename, t5_click_qa, t6_log_rootcause, t7_git_history, t8_control_edit)

| arm | n | pass | mean cost $ | cost/correct $ | turns | input | cache write | cache read | output | wall s | caps | timeouts |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| baseline | 80 | 80/80 | 0.0454 | 0.0454 | 6.1 | 12 | 6119 | 123447 | 857 | 21 | 0 | 0 |
| ojas3 | 80 | 80/80 | 0.0441 | 0.0441 | 6.0 | 12 | 5965 | 118873 | 837 | 19 | 0 | 0 |
| headroom | 80 | 80/80 | 0.0423 | 0.0423 | 5.5 | 11 | 6032 | 104810 | 765 | 28 | 0 | 0 |
| rtk | 80 | 80/80 | 0.0475 | 0.0475 | 6.1 | 12 | 6669 | 122839 | 849 | 18 | 0 | 0 |

Ratio vs baseline, task-balanced means, 95% stratified bootstrap interval; pass rate Fisher exact p:

| arm | cost | turns | cache read | cache write | output | pass rate (p) |
|---|---|---|---|---|---|---|
| ojas3 | 0.97x [0.89, 1.06] | 0.98x [0.91, 1.06] | 0.96x [0.86, 1.07] | 0.97x [0.90, 1.06] | 0.98x [0.87, 1.10] | 80/80 vs 80/80 (p=1.000) |
| headroom | 0.93x [0.85, 1.03] | 0.91x [0.84, 1.00] | 0.85x [0.75, 0.98] | 0.99x [0.89, 1.10] | 0.89x [0.79, 1.02] | 80/80 vs 80/80 (p=1.000) |
| rtk | 1.05x [0.97, 1.13] | 1.01x [0.93, 1.09] | 1.00x [0.89, 1.11] | 1.09x [1.01, 1.18] | 0.99x [0.89, 1.11] | 80/80 vs 80/80 (p=1.000) |

Per task: mean cost $ / mean turns (passes/n)

| task | baseline | ojas3 | headroom | rtk |
|---|---|---|---|---|
| t1_click_bug | 0.0616 / 7.8 (10/10) | 0.0611 / 7.3 (10/10) | 0.0636 / 8.2 (10/10) | 0.0715 / 9.7 (10/10) |
| t2_express_bug | 0.0602 / 8.3 (10/10) | 0.0664 / 9.3 (10/10) | 0.0504 / 7.1 (10/10) | 0.0598 / 8.1 (10/10) |
| t3_walkdir_bug | 0.1423 / 16.9 (10/10) | 0.1309 / 15.2 (10/10) | 0.1152 / 13.1 (10/10) | 0.1329 / 14.8 (10/10) |
| t4_click_rename | 0.0169 / 3.5 (10/10) | 0.0158 / 3.2 (10/10) | 0.0220 / 3.8 (10/10) | 0.0220 / 4.0 (10/10) |
| t5_click_qa | 0.0132 / 3.5 (10/10) | 0.0177 / 4.0 (10/10) | 0.0216 / 3.4 (10/10) | 0.0172 / 3.7 (10/10) |
| t6_log_rootcause | 0.0352 / 3.4 (10/10) | 0.0215 / 3.4 (10/10) | 0.0258 / 3.4 (10/10) | 0.0346 / 3.2 (10/10) |
| t7_git_history | 0.0234 / 2.2 (10/10) | 0.0247 / 2.2 (10/10) | 0.0242 / 2.3 (10/10) | 0.0259 / 2.4 (10/10) |
| t8_control_edit | 0.0106 / 3.0 (10/10) | 0.0150 / 3.0 (10/10) | 0.0155 / 3.0 (10/10) | 0.0158 / 3.0 (10/10) |

Tool activity: ojas3: ojas_stats_entries total=109 (runs>0: 58/80), ojas_read_tool_outlines total=0 (runs>0: 0/80), ojas_grep_tool total=0 (runs>0: 0/80); rtk: rtk_commands total=372 (runs>0: 78/80)

## Block: long  (120 runs, tasks: l1_click_duration, l2_express_ci, l3_click_split_core)

| arm | n | pass | mean cost $ | cost/correct $ | turns | input | cache write | cache read | output | wall s | caps | timeouts |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| baseline | 30 | 29/30 | 0.0998 | 0.1033 | 9.0 | 18 | 11097 | 206209 | 3477 | 39 | 0 | 0 |
| ojas3 | 30 | 30/30 | 0.1063 | 0.1063 | 8.8 | 18 | 12049 | 206114 | 3744 | 44 | 0 | 0 |
| headroom | 30 | 30/30 | 0.1016 | 0.1016 | 9.0 | 18 | 11318 | 200276 | 3631 | 54 | 0 | 0 |
| rtk | 30 | 30/30 | 0.1060 | 0.1060 | 9.2 | 18 | 11588 | 218683 | 3776 | 41 | 0 | 0 |

Ratio vs baseline, task-balanced means, 95% stratified bootstrap interval; pass rate Fisher exact p:

| arm | cost | turns | cache read | cache write | output | pass rate (p) |
|---|---|---|---|---|---|---|
| ojas3 | 1.06x [0.97, 1.18] | 0.98x [0.89, 1.09] | 1.00x [0.88, 1.14] | 1.09x [0.98, 1.20] | 1.08x [0.96, 1.21] | 30/30 vs 29/30 (p=1.000) |
| headroom | 1.02x [0.92, 1.13] | 1.00x [0.90, 1.11] | 0.97x [0.84, 1.11] | 1.02x [0.92, 1.13] | 1.04x [0.92, 1.18] | 30/30 vs 29/30 (p=1.000) |
| rtk | 1.06x [0.97, 1.17] | 1.02x [0.93, 1.12] | 1.06x [0.94, 1.20] | 1.04x [0.95, 1.15] | 1.09x [0.98, 1.21] | 30/30 vs 29/30 (p=1.000) |

Per task: mean cost $ / mean turns (passes/n)

| task | baseline | ojas3 | headroom | rtk |
|---|---|---|---|---|
| l1_click_duration | 0.0714 / 4.9 (9/10) | 0.0858 / 5.5 (10/10) | 0.0768 / 5.0 (10/10) | 0.0748 / 4.4 (10/10) |
| l2_express_ci | 0.0735 / 6.6 (10/10) | 0.0715 / 5.8 (10/10) | 0.0722 / 6.5 (10/10) | 0.0606 / 5.6 (10/10) |
| l3_click_split_core | 0.1546 / 15.5 (10/10) | 0.1615 / 15.2 (10/10) | 0.1559 / 15.5 (10/10) | 0.1826 / 17.6 (10/10) |

Tool activity: ojas3: ojas_stats_entries total=24 (runs>0: 16/30), ojas_read_tool_outlines total=0 (runs>0: 0/30), ojas_grep_tool total=0 (runs>0: 0/30); rtk: rtk_commands total=205 (runs>0: 30/30)

Invalid runs (excluded): none
Incomplete dirs (in progress or failed infra): none

