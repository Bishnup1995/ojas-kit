# Follow-up 3: pre-registration (committed 2026-11-23, before the first run)

Two studies, run back to back by `v2/run_study.sh` (same harness as v2; result events merged as in DEVIATIONS.md).
Analysis: `v2/analyze_v2.py` (task-balanced ratios vs baseline, stratified bootstrap 5,000 resamples seed 1,
Fisher exact for pass rate). Every run is reported. Setups: no tool (`ojasbench/baseline:2`) vs ojas v0.3
(`ojasbench/ojas3fix:2`, main 6e42815). Claude Code 2.1.329. Arm order alternates by repeat.

## Study A: long3, the on-call day (results/v3-long3)

Task `l6_oncall_day`: one session, three repos, five jobs, each an already-validated task (the l4 click sprint,
the l5 express CI failure with 4 causes, the t3 walkdir bug). Grader validated: start FAIL, good PASS, bad FAIL
(results/v2/validation_long3.txt). Model claude-sonnet-5-5, 5 repeats per setup (10 runs), up to 150 turns and
120 minutes per run.

- Validity check: median turns >= 25 for both setups, else the study is reported as "not long enough".
- H4c: in such sessions ojas costs less than no tool (cost ratio < 1 with the interval excluding 1.0).
- H5c: ojas's pass rate is not lower.
- Decision rule: an ojas fix only if ojas costs more with the interval excluding 1.0, or passes fewer runs.
- Also reported (descriptive, for the "show only what changed between test runs" idea): the share of tool-output
  tokens that comes from repeated runs of the same test command. Build that feature only if, in the no-tool runs,
  repeats make up at least 25% of tool-output tokens.

## Study B: second model (results/v3-haiku)

Model claude-haiku-5-5. The 4 big-output tasks (s1-s4) and the 8 ordinary tasks (t1-t8), 5 repeats per setup
(120 runs), v2 limits (60 turns, 30 minutes).

- H2h: on big-output tasks ojas costs less than no tool (interval excludes 1.0).
- H3h: on ordinary tasks ojas is within 10% of no tool.
- H5h: ojas's pass rate is not lower in either group.
- These test whether the Sonnet results carry over to another model. Not pooled with v2.

## Not run: short bodies in outlines

Measured before any run: every function in the two generated large files (r1, r2) has a 15-22 line body, and
none has 3 lines or fewer; in click's core.py (r3) 19 of 136 bodies have 3 lines or fewer and the injected bug is
in none of them. Showing short bodies in full cannot change these tasks, so no runs are spent on it.
