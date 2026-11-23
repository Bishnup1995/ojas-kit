# Haiku check of `ojas read --full` (exploratory; v2/DEVIATIONS.md, 2026-11-23)

10 runs of s3_click_core_full_read, ojas arm only, ojasbench/ojasnote:3 (ojas-kit 14fec07).

- Passed 9/10 (v3-haiku-note: 4/5; baseline 5/5 in both earlier Haiku runs).
- 6 runs used `ojas read <file> --full` after the outline; 2 more used `--symbol` only. No run used OJAS_RAW.
- Run #2 failed: after the outline it stopped and asked the user which way to read the file, without trying `--full`.
- Cost: median $0.0048 (baseline s3 mean $0.0149 in v3-haiku-note), but mean $0.0196: runs #1 and #4 also opened the full output with the Read tool and cost about $0.08 each.
