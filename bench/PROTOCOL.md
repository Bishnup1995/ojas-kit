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

- (none yet)
