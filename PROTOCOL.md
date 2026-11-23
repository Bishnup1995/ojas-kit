# Benchmark protocol (pre-registered)

Committed **before** any result was collected. Changes after the first run are recorded in
`CHANGELOG` at the bottom, with the reason. Results are never dropped; failed and
inconclusive runs are published.

## Conflict of interest

ojas is written by the person running this benchmark. The design below exists to limit
the effect of that: pinned versions, identical tasks and graders, randomised order, raw
data published, and every arm installed as its own authors document it, not as
we would tune it. Anyone can rerun everything with `bench/run.sh`.

## What is compared

| Arm | What it is | Layer it acts on | Pinned version |
|---|---|---|---|
| `baseline` | Claude Code, nothing else | none | claude-code 2.1.329 |
| `ojas` | PreToolUse/SessionStart hooks, rule-based filters | tool output (command rewrite) + ~150 tok session rules | commit a46b045 |
| `caveman` | Default full install: say-less voice, read-less, blocks, waste fixes | output style + tool output + input | v4.1.0 |
| `headroom` | `headroom wrap claude`: local proxy | the API request (all messages) | headroom-ai 0.44.0 |

These are different kinds of tool. The comparison is about what a user gets from each as
shipped, not about which algorithm is best in isolation. Caveman's Auto routing needs
an account and changes the model, so it is **off** (it would break the fixed-model rule).

## Isolation

- One Docker image per arm, built from a shared base. Same OS, toolchains, repo
  snapshots (pinned by commit SHA), model, and Claude Code version.
- Every run starts a **fresh container** with its own `HOME`. No state (hook stores,
  caches, Claude history, headroom memory) survives between runs.
- Arms cannot see each other's files. The only shared thing is the network path to
  the API.
- Each arm's install is verified by a smoke test that proves the tool is active
  (see `bench/smoke.sh`). An arm whose tool is not demonstrably active in a run is
  marked invalid, not scored.

## Layer 1: offline compression benchmark (deterministic)

Replays identical recorded tool outputs through each tool's compressor. No model calls.

- Corpus: raw output of ~30 commands run in the pinned repos (test runs, grep, find, git
  diff/log/show, build logs, JSON API output, file reads).
- Metric 1, size: tokens before/after, counted with tiktoken `o200k_base` for every arm
  (a neutral tokenizer; absolute numbers differ from Claude's tokenizer, ratios are what
  matter).
- Metric 2, fidelity: each corpus item has a list of "must survive" facts (failing test
  names, error lines, a planted needle). A fact counts as kept if it is in the compressed
  output, or the compressed output gives a retrieval handle that returns it.
- Not every tool applies to every item (e.g. caveman's voice rules do not touch tool
  output). "Not applicable" is reported as such, not as 0%.

## Layer 2: end-to-end agent runs

- Model: fixed single model for all arms (`claude-sonnet-5-5`), default effort.
- Driver: `claude -p --output-format json`, the same prompt text for every arm.
- Tasks: 8 tasks over pinned repos, each with an automatic pass/fail grader that runs
  outside the agent (the agent never sees the grader). Task list and graders are in
  `bench/layer2/tasks/` and are frozen before the first run.
- Repeats: 5 per (task, arm). Run order is randomised with a recorded seed; arms are
  interleaved so time-of-day and rate-limit effects hit all arms alike.
- Auth: a Claude subscription. The API reports token usage but not a bill, so **cost is
  computed from tokens at published API list prices** (input, cache write, cache read,
  output). Subscription use means no per-run dollar charge was actually incurred.
- Metrics, per arm:
  1. Pass rate (primary quality metric).
  2. Total cost at list price per run, and per **passed** run.
  3. Tokens by type: input, cache write, cache read, output.
  4. Number of turns and tool calls (to catch outline-style "extra round trips").
  5. Wall-clock time.
- Statistics: per-task paired comparison against baseline, bootstrap 95% intervals over
  runs. With n=5 per cell, small differences will not be significant, and the write-up
  says so rather than ranking arms on noise.

## Rules to prevent bias

1. Pin everything; record versions and image digests in each result file.
2. Default configuration for each arm, as documented by its authors. No hand-tuning of
   any arm, including ojas, between runs.
3. Same task prompt for all arms. No arm-specific hints.
4. Grader is arm-blind and runs outside the agent.
5. A bug found in the harness is fixed for all arms and affected runs are re-run for all
   arms. A bug found in one tool is reported, not patched.
6. All results published, including arms that win where ojas loses.
7. Report what each tool is *not* good at.

## Known limits (stated up front)

- n=5 per cell gives wide intervals. This is a small benchmark, not a leaderboard.
- Tasks are chosen to include large tool output because that is where all three tools claim
  gains. Tasks with small outputs would show smaller differences. The write-up reports the
  task mix honestly.
- Subscription-based runs can be rate-limited; a throttled run is retried, not dropped.
- Tokens are counted by two methods (neutral tokenizer in Layer 1, API-reported in Layer 2).

## CHANGELOG

- 2026-11-20, Layer 1 scorer fix (after first scoring, before any Layer 2 run). The first
  "fact line" regex matched any line containing words like `failed`, so it flagged incidental
  source text (e.g. `cat core.py`) and made ojas look lossy (125/168 facts kept). That was a
  metric bug, not a tool result. Fact lines are now only primary failure/summary identifiers
  (`FAILED ...`, `N failing`, `test ... FAILED`, `panicked at`, `test result:`, FATAL, ...),
  applied identically to every arm. The first run's numbers are kept in git history.
- 2026-11-20, headroom arm: its Kompress text model is pre-downloaded into the image (it only
  compresses text when the model is present). Its 20 s inline time budget still expires on
  some large items on this 8-CPU container; those pass through uncompressed and are reported.
- 2026-11-20, Layer 1 `caveman` arm uses `caveman-engine compress` (stdin to stdout), the
  engine the product's own hooks call; ojas uses `ojas filter --kind K` / `ojas read` with K
  chosen by item name the way ojas's own command classifier would.
- 2026-11-20, Layer 2 repeats: first pass is 3 repeats per (task, arm), 96 runs, chosen to fit the
  subscription's usage window. If time and limits allow it is extended to 5 (runs resume by
  folder name; the extension re-shuffles order with the same seed). Pilot (1 run of t8 per arm)
  found and fixed two harness bugs before any scored run: the t8 grader wrongly required the
  `console.log` text to change, and `headroom wrap` prints a banner before the JSON result.
  Both fixes apply to all arms. The 4 pilot t8 runs (after the fixes) count as t8 repeat 1.
- 2026-11-20, Layer 2 harness bugs found mid-run (affect all arms equally, fixed, no scored data lost):
  (1) The throttle detector matched the substring "429" (and similar) anywhere in run.json, so
  ordinary numbers such as `duration_api_ms":4291` triggered a 15-minute sleep and a re-run of a valid
  run (6 times). It now fires only when Claude Code returns `is_error: true` with a limit message.
  The discarded runs were valid and complete, and each was re-run once; arm identity did not
  influence which runs were hit.
  (2) After the first run was reported killed for low memory, its runner process survived, and a second
  runner was started on resume, so two runners worked the same schedule concurrently for about an
  hour. Both were stopped. All 65 run folders were checked for a single transcript matching the
  run's session id; 2 folders (headroom and ojas, t2 repeat 3) had empty results and were deleted
  and re-run. The other 63 are consistent.
- 2026-11-20, STRESS SET (post-hoc, added after the main Layer 2 result, written here before any stress run).
  The main set (96 runs) found no cost saving for any arm: agents kept outputs small by themselves and
  sessions cost about $0.04, so there was little to compress. To test the regime where these tools
  claim gains, 4 extra tasks (s1 to s4) use prompts that deliberately force large, unfiltered output
  (full pytest run, full `npm test`, `cat` of a 4,000-line file, `cat` of a 20,000-line log). 3 repeats
  per arm, 48 runs, same arms, model, isolation and graders-outside-the-agent rule.
  This is reported as a separate experiment, never merged into the main table, and its prompts are
  not representative of normal agent behaviour: they measure the best case for output compression.
  Fixed before running: tasks, prompts and graders as in `bench/layer2/tasks.sh` at this commit.

