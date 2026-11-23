# Benchmark v2: pre-registration

Written and committed **before** any scored v2 run. Smoke runs (one per arm, `results/v2/smoke/`) exist only to
check the harness and are never scored. Anything changed after the first scored run is recorded in
`DEVIATIONS.md` with the date and reason. No tasks will be added after the fact. All runs are published,
including failures. The conflict of interest stated in `bench/PROTOCOL.md` still applies: ojas's author runs
this benchmark.

## Questions and hypotheses

v1 found no saving on ordinary short tasks and a saving for ojas (v1 build) only when prompts forced large
output. Since then ojas gained a PostToolUse hook (Bash, Grep and Read output) and outlines for large whole-file
Reads, and dropped its response-style text by default. v2 asks:

- **H1 (read block).** On tasks that need large source files, ojas3 costs less than baseline. *Competing
  expectation, stated up front:* outlines can cost extra turns when the agent needs whole bodies (task r2 is built
  to test this), so H1 may fail on r2 or on the block.
- **H2 (stress block).** On forced-large-output tasks, ojas3 costs less than baseline (v1: 0.77x for ojas).
- **H3 (main block).** On ordinary tasks, no arm differs from baseline by more than 10% in cost. (v1: all
  intervals touched or included 1.0 except caveman.)
- **H4 (long block).** In 25+ turn sessions, savings from output compression accumulate through cache reads, so
  ojas3, rtk and headroom cost less than baseline.
- **H5 (quality).** No arm has a lower pass rate than baseline in any block.

These are directional expectations, not claims. Every block is reported whatever the outcome.

## Arms (fresh container per run, shared base image `ojasbench/baseline:2`)

| Arm | What | Version | Install |
|---|---|---|---|
| `baseline` | Claude Code only | claude-code 2.1.329 | unchanged v1 image |
| `ojas3` | ojas from the ojas repo `main` | commit abf4776 (`git archive main`, built with Rust 1.87 `--locked`) | `ojas init`, default config: PreToolUse(Bash), PostToolUse(Bash\|Grep\|Read), SessionStart; style off |
| `headroom` | `headroom wrap claude` proxy | headroom-ai 0.44.0 | unchanged v1 image |
| `rtk` | rtk-ai/rtk | v1.2.1 release binary `rtk-x86_64-unknown-linux-musl`, checksum verified | `rtk init -g --auto-patch` (README's Claude Code setup): PreToolUse(Bash) `rtk hook claude`, `~/.claude/RTK.md` + `@RTK.md` in `~/.claude/CLAUDE.md` |

caveman is dropped. Model: `claude-sonnet-5-5` for every arm, default effort. Driver:
`claude -p <prompt> --output-format stream-json --verbose --dangerously-skip-permissions --max-turns 60`
(stream-json so the init event shows the auth path; the final result object is the same as `--output-format json`).
Wall-clock timeout 1,800 s (3,600 s in the long block). Auth: the user's Claude subscription
(`CLAUDE_CODE_OAUTH_TOKEN`); the runner aborts if an API key is present anywhere or a run reports
`apiKeySource` other than `none`.

**Isolation change vs v1 (applies to all arms):** task definitions, graders and fixture generators are mounted
under `/root` (unreadable to the agent's user) and copied to a temp dir only for setup and grading. In v1 the
agent could in principle read `/tasks.sh`.

## Tasks (18 tasks, 4 blocks, run in this order)

| Block | Task | What the agent does | Grader (deterministic, outside the agent) |
|---|---|---|---|
| read | r1_pricing_policy | Generated 2,726-line `pricing.py` + README policy; one boundary bug (line 2276, `qty > 100` vs policy "100 or more") | agent's module must equal the correct reference on every `rate_*` function and on `quote()` over a fixed grid (boundaries 99/100/101/499/500/... , 3 loyalty levels, 3 regions) |
| read | r2_ledger_raises | Generated 2,580-line `ledger.py`, 124 functions: list every function that can raise `LedgerError` directly or transitively, by reading only (no execution or scripts). Needs whole bodies: try/except guards, an `except LedgerWarning` that does not catch, an `except Exception` that does, comment/docstring decoys. Outlines are expected to hurt here. | exact set match (15 names, cross-checked by an independent AST analysis) |
| read | r3_click_core_bug | Bug injected in click `core.py` (4,018 lines) line ~3470: `Option._parse_decls` sorts short option names first (history hidden) | click test suite passes (tests restored from git; `tests/test_utils` ignored as in v1) |
| stress | s1-s4 | v1 stress tasks, unchanged prompts and graders | as v1 |
| main | t1-t8 | v1 main tasks, unchanged prompts and graders | as v1 |
| long | l1_click_duration | Implement `click.Duration` (spec in prompt) + tests + docs + changelog | hidden acceptance tests (parsing, invalid input, min/max, CLI) + full suite + docs/tests/CHANGES checks |
| long | l2_express_ci | Three injected bugs in express `lib/` (request.js, utils.js, response.js; history hidden), plus a 4,388-line (337 KB) CI log of the failing job (npm install noise + full mocha output) | express test suite passes (tests restored from git) |
| long | l3_click_split_core | Move `Parameter`, `Option`, `Argument` from `core.py` into new `click/params.py`, keep all import paths working | classes defined only in params.py; identity checks across `click`, `click.core`, `click.params`; full suite (tests restored) |

Repeats: 10 per (task, arm). 4 arms x (30 + 40 + 80 + 30) = **720 runs**. Within each block, jobs (arm, task, rep)
are interleaved and shuffled with a fixed seed (read 2026112101, stress 2026112102, main 2026112103, long
2026112104); the schedule is written to `results/v2/schedule.txt`.

Grader validation (`bench/v2/validate_v2.sh`, results in `results/v2/validation.txt`): for each new task the start
state FAILs, a known-good solution PASSes and a known-bad solution FAILs; v1 tasks are re-checked (start FAIL,
known-good PASS) in the new staging setup.

## Per-run validity (arm activity), from `verify.txt`

- all arms: init `apiKeySource` = `none` (else the runner aborts).
- baseline: no ojas/rtk hooks, no ojas session note.
- ojas3: PostToolUse `Bash|Grep|Read` hook installed, ojas SessionStart note present in the transcript, **no**
  "Response style" text. Compression counts (`stats.jsonl`: entries, Read-tool outlines, Grep-tool) are reported.
- rtk: `rtk hook claude` installed and no ojas hook; `rtk gain` command count reported. A run in which the agent
  ran no rewritable command is still valid (the tool was installed and active; it had nothing to do).
- headroom: no ojas/rtk hooks; headroom present in the wrapper output or `headroom perf`.
Runs failing their check are excluded from the metrics and listed.

## Metrics

Per run: pass/fail; Claude Code's list-price cost estimate `total_cost_usd` (not a bill; subscription use);
turns; input, cache-write, cache-read and output tokens; wall time; whether `--max-turns` was hit
(`error_max_turns`, counted as a run and graded normally); timeout (scored as fail, no token data).
Derived: cost per correct answer = total cost / passes.

## Analysis (`bench/v2/analyze_v2.py`), per block, never pooled across blocks

- Ratio vs baseline for cost (primary), turns, cache read, cache write and output tokens: ratio of task-balanced
  means (mean over tasks of the per-task mean), 95% interval from a bootstrap stratified by task (runs resampled
  within task and arm), 5,000 resamples, seed 1. An interval excluding 1.0 is called a difference.
- Pass rate vs baseline: two-sided Fisher exact test per block.
- Cost per correct answer per arm.
- No multiple-comparison correction; with 3 comparisons x 4 blocks, single p-values near 0.05 are treated as weak.

## Rate limits and failures

A run that fails because of a usage/rate limit (error result with status 429 or limit text, or limit text on
stderr) is deleted and retried after the reported reset (else 30 min); pauses go to `results/v2/pauses.log`.
Other infrastructure failures (no result object, API 5xx/overloaded) are retried after 5 min, at most 6 times in
a row, then the runner stops. Neither kind is ever scored. Only one container runs at a time.
