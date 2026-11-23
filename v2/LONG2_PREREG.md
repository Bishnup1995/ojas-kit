# Benchmark v2 follow-up: long sessions (pre-registration)

Written 2026-11-23, before any scored run of this study. All runs are reported. Any change after the first
scored run goes in `DEVIATIONS.md` with date and reason.

## Why

H4 of the main study ("in 25+ turn sessions, savings accumulate through cache reads") was not tested. The long
block's tasks finished in a median of 5, 6 and 16 turns, so nothing could accumulate. This study uses tasks
built so they cannot end in a few turns.

## Hypotheses

- **H4b.** In sessions with a median of at least 25 turns, ojas costs less than no tool (cost ratio below 1.0).
  *Competing expectation:* ojas only saves where a large output occurs, so the effect may be small or zero.
- **H5b.** ojas's pass rate is not lower than no tool's.

## Arms

- `baseline`: image `ojasbench/baseline:2`, unchanged from the main study.
- `ojas3`: image `ojasbench/ojas3fix:2`, i.e. ojas built from the ojas repo's main at 6e42815 (v0.3: PR #8 fixes),
  installed with `ojas init`, default config. It is the same image as the exploratory extra arm.

headroom and rtk are left out to fit the study within the Claude Pro plan's usage limits.

## Tasks (block `long2`)

Each task chains existing tasks whose graders were already validated in the main study.

| Task | What the agent does | Grader |
|---|---|---|
| l4_click_sprint | In one session, in click: (1) fix the failing test suite (the hidden t1 and r3 bugs), (2) implement `click.Duration` with tests, docs and changelog (l1 spec), (3) move Parameter/Option/Argument into `params.py` (l3 spec). Run the whole suite after each job. | l1 grader (docs, changelog, tests, hidden Duration tests, full suite), then the l3 grader (module layout, import identity, full suite) |
| l5_express_ci_plus | Red CI log (`ci.log`) caused by 4 independent bugs (the 3 l2 bugs plus the t2 bug). Fix one at a time and rerun `npm test` after each fix. | full express suite (t2 grader) |

Grader validation (`validate_v2.sh` with `VALIDATION_OUT=../results/v2/validation_long2.txt`): the start state
fails, a known-good solution passes, and a known-bad solution fails (l4: one bug left in; l5: 3 of 4 bugs fixed).

## Runs

5 repeats per (task, arm): 2 tasks × 2 arms × 5 = **20 runs**. The schedule interleaves arms (alternating which
arm goes first in each repeat). Model `claude-sonnet-5-5`, at most 100 turns, 5,400 s wall-clock timeout, the
user's Claude subscription only (the same auth guards as the main study). Results go in `results/v2-long2/`.

## Analysis

The method is the same as the main study: ratio of task-balanced means against baseline for cost (primary),
turns, cache read, cache write and output; 95% bootstrap interval stratified by task (5,000 resamples, seed 1);
Fisher exact test for pass rate.

**Validity check:** for each task, report the median turns per arm. A task counts as a long session only if both
arms have a median of 25 turns or more. If it does not, the result for that task is reported as "not long enough"
and H4b is not judged on it.

**Decision rule for ojas (stated in advance):** if ojas costs more than baseline with an interval that excludes
1.0, or has a lower pass rate, we look for the cause in the transcripts (for example, repeated test-suite output)
and fix it in a separate PR. Otherwise no fix is made.

With 5 repeats the intervals will be wide. A result whose interval includes 1.0 is reported as no difference.
