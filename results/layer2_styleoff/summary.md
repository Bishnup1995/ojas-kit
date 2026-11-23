| arm | runs | pass | mean cost $ | mean turns | mean input | mean cache-write | mean cache-read | mean output | mean wall s |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| baseline | 24 | 24/24 | 0.0461 | 5.96 | 12 | 6575 | 116191 | 817 | 26 |
| ojas | 24 | 24/24 | 0.0520 | 6.58 | 13 | 7148 | 137713 | 966 | 31 |
| ojas-off | 24 | 24/24 | 0.0454 | 5.71 | 11 | 6585 | 113448 | 767 | 18 |

Per task, mean cost $ / mean turns (pass):

| task | baseline | ojas | ojas-off |
|---|---:|---:|---:|
| t1_click_bug | 0.0551 / 6.3 (3/3) | 0.0601 / 7.7 (3/3) | 0.0646 / 7.7 (3/3) |
| t2_express_bug | 0.0951 / 13.7 (3/3) | 0.0982 / 12.7 (3/3) | 0.0719 / 9.7 (3/3) |
| t3_walkdir_bug | 0.1012 / 11.7 (3/3) | 0.1477 / 17.3 (3/3) | 0.1173 / 13.3 (3/3) |
| t4_click_rename | 0.0217 / 4.3 (3/3) | 0.0217 / 3.3 (3/3) | 0.0160 / 3.0 (3/3) |
| t5_click_qa | 0.0165 / 3.0 (3/3) | 0.0174 / 3.7 (3/3) | 0.0179 / 3.7 (3/3) |
| t6_log_rootcause | 0.0355 / 3.3 (3/3) | 0.0321 / 3.0 (3/3) | 0.0352 / 3.0 (3/3) |
| t7_git_history | 0.0260 / 2.3 (3/3) | 0.0246 / 2.0 (3/3) | 0.0253 / 2.3 (3/3) |
| t8_control_edit | 0.0178 / 3.0 (3/3) | 0.0147 / 3.0 (3/3) | 0.0149 / 3.0 (3/3) |

Cost ratio vs baseline (paired bootstrap over task means, 95% interval):

- ojas / baseline: 1.04x  [0.93, 1.17]  over 8 tasks
- ojas-off / baseline: 0.96x  [0.85, 1.07]  over 8 tasks
- ojas / ojas-off: 1.09x  [0.97, 1.23]  over 8 tasks

ojas compression (tool_stats.txt, summed over runs): ojas: 0 compressed of 38 commands through ojas, ojas-off: 0 compressed of 39 commands through ojas

Total estimated cost: $3.4449

Verification problems: none (baseline: no ojas text; ojas: style+note; ojas-off: note only)
