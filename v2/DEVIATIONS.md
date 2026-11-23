# Benchmark v2: deviations from PREREGISTRATION.md

Changes made after the first scored run, with date and reason. Empty means none.

## 2026-11-23: extra exploratory arm `ojas3fix` (after all 720 pre-registered runs were complete)

- **What:** the read block (r1, r2, r3 × 10 repeats = 30 runs) run once more with ojas built from the ojas repo's
  main at 6e42815 (PR #8). That build leaves output the agent already narrowed alone, and returns full text on a
  repeated whole-file Read. Image `ojasbench/ojas3fix:2`, same Dockerfile and base as `ojas3`. Runner
  `run_extra_v2.sh` (a copy of `run_v2.sh` that differs only in output dir, image and schedule). Results go in
  `results/v2-extra/`.
- **Why:** the read-block results (ojas3 run r2 #4 failed; r2 cost 1.48x) pointed to the two problems these
  fixes address. This arm measures whether the fixes help.
- **Status:** exploratory, not pre-registered. It is reported separately and never pooled into the
  pre-registered tables. It is compared against the pre-registered baseline and ojas3 read-block runs, which
  were run a day earlier, so time effects can't be excluded.

## Merging result events (2026-11-23)

When a background task the agent started ends after the task, Claude Code emits a second `result` event. run.json kept only the last event, whose num_turns, usage and duration_ms cover only that last segment (total_cost_usd is cumulative and was always right). This hit 7 t2 runs in the main study and 2 l5 runs in long2. analyze_v2.py now sums those fields over all result events in run.raw, and run_one_v2.sh merges them the same way for future runs. Costs and pass rates do not change. Main-block turn and token ratios shift slightly; the recomputed table is results/v2/summary_merged.md (summary.md is kept as originally produced).

## 2026-11-23: Haiku confirmation of the session-note fix (after follow-up 3 was complete)

- **What:** the follow-up 3 Haiku study (`results/v3-haiku`, 120 runs) run again with the same schedule, model, tasks and baseline image. The only change is the ojas arm's image: `ojasbench/ojasnote:2`, built from ojas-kit commit af94fe5 (v0.4-dev). That commit rewords the session note so it no longer advertises `OJAS_RAW=1`, and says Claude Code cuts raw Bash output over about 30k characters. The arm keeps the name `ojas3` so run_one_v2.sh treats it the same. Results go in `results/v3-haiku-note/`.
- **Why:** in v3-haiku, Haiku prefixed commands with `OJAS_RAW=1` in 18 of 60 ojas runs (4 of the 5 failed s2 runs), and ojas did not help (stress 1.07x, normal 1.17x). This measures whether the reworded note removes the bypass and restores the Sonnet result.
- **Status:** exploratory, not pre-registered. Analysed with the same FOLLOWUP3_PREREG.md study-B rules and reported next to v3-haiku, never pooled with it. Baseline is re-run, so the comparison is within this run set.

## 2026-11-23: Haiku check of `ojas read --full` in the session note

- **What:** 10 Haiku runs of s3_click_core_full_read with the ojas arm only, image `ojasbench/ojasnote:3` (ojas-kit 14fec07: the note now names `ojas read <file> --full` as the way to see a whole large file in one read). Results in `results/v3-haiku-full/`.
- **Why:** in v3-haiku-note, s3 run #5 failed: `cat` of the 4,018-line file returned an outline, the task forbids other reads, and the note no longer offered a raw escape.
- **Status:** exploratory check of one task; no baseline arm (baseline s3 was 5/5 in both v3-haiku and v3-haiku-note).
