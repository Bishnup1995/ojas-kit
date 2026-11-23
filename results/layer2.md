| arm | runs | pass rate | mean cost $ | cost per pass $ | mean turns | mean cache-read tok | mean output tok | mean wall s |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| baseline | 24 | 100% | 0.043 | 0.043 | 5.6 | 107414 | 775 | 18 |
| ojas | 24 | 100% | 0.048 | 0.048 | 5.9 | 117236 | 766 | 13 |
| caveman | 24 | 100% | 0.055 | 0.055 | 5.7 | 118716 | 822 | 20 |
| headroom | 24 | 100% | 0.042 | 0.042 | 5.4 | 101163 | 725 | 21 |

Per task, mean cost $ (pass rate):

| task | baseline | ojas | caveman | headroom |
|---|---:|---:|---:|---:|
| t1_click_bug | 0.065 (3/3) | 0.069 (3/3) | 0.086 (3/3) | 0.060 (3/3) |
| t2_express_bug | 0.049 (3/3) | 0.082 (3/3) | 0.065 (3/3) | 0.053 (3/3) |
| t3_walkdir_bug | 0.115 (3/3) | 0.121 (3/3) | 0.126 (3/3) | 0.120 (3/3) |
| t4_click_rename | 0.021 (3/3) | 0.022 (3/3) | 0.029 (3/3) | 0.020 (3/3) |
| t5_click_qa | 0.019 (3/3) | 0.019 (3/3) | 0.020 (3/3) | 0.017 (3/3) |
| t6_log_rootcause | 0.038 (3/3) | 0.033 (3/3) | 0.056 (3/3) | 0.037 (3/3) |
| t7_git_history | 0.025 (3/3) | 0.024 (3/3) | 0.035 (3/3) | 0.023 (3/3) |
| t8_control_edit | 0.014 (3/3) | 0.015 (3/3) | 0.021 (3/3) | 0.010 (3/3) |

Cost ratio vs baseline (paired bootstrap over task means, 95% interval):

- ojas: 1.09x  [0.97, 1.28]  over 8 tasks
- caveman: 1.32x  [1.21, 1.41]  over 8 tasks
- headroom: 0.93x  [0.84, 1.01]  over 8 tasks
