# Benchmark plan (for review; nothing has been run)

Companion to `PROTOCOL.md` (the pre-registered rules). This file is the work plan and the
decisions I need from you before any test runs.

## Status

| Item | State |
|---|---|
| Docker images for 4 arms (`bench/docker/Dockerfile`, `build.sh`) | built, pinned |
| Offline smoke test (`bench/smoke.sh`, no model calls) | passing for all 4 arms |
| Protocol | drafted (`PROTOCOL.md`) |
| Task repos + graders | **not started** (needs your sign-off on the task list below) |
| Layer 1 corpus + scorer | not started |
| Layer 2 runner (`run.sh`) + cost/usage parser | not started |
| Any benchmark run | **none, waiting for your go** |

## Arms as installed (all verified by smoke test)

| Arm | Pin | How it activates in a run |
|---|---|---|
| baseline | claude-code 2.1.329 | `claude -p ...` |
| ojas | commit a46b045, built in-image | hooks via `ojas init`; `claude -p ...` |
| caveman | v4.1.0, plugin + `cave setup --yes` (proxy, hooks, MCP, blocks) | start runtime (`caveman doctor claude --fix`), then `claude -p ...`. Setup sets `ANTHROPIC_BASE_URL=http://127.0.0.1:8787/w/claude`. Auto routing inactive (needs login) |
| headroom | headroom-ai 0.44.0 `[all]` | `headroom wrap claude -- -p ...` (starts proxy on 8787) |

Image digests are recorded in every result file.

## Layer 1: offline compression (free, deterministic)

~30 recorded outputs: pytest/cargo/jest failures, large grep, find, ls -l, git log/diff/show,
npm install, docker/CI logs, big JSON, plus whole-file reads. Each replayed through:
ojas (`ojas filter`/`ojas read`), headroom (`compress()` library, same engine as proxy),
caveman (its compressor engine on the same bytes). Scored on tokens saved (o200k_base) and
on whether a list of must-survive facts per item is kept or retrievable.
Caveman's voice rules have no tool-output effect; reported "n/a", not 0%.

## Layer 2: end-to-end (uses your subscription)

Draft task set (8), each on a pinned public repo, grader runs outside the agent:

| # | Task | Output-heavy part | Grader |
|---|---|---|---|
| 1 | Fix an injected bug, pytest suite | long test output | hidden tests pass |
| 2 | Fix a failing Rust test | cargo build/test noise | `cargo test` passes |
| 3 | Fix a failing JS test | npm test output | npm test passes |
| 4 | Rename a function across a repo | big grep | tests pass + no old name |
| 5 | Q&A: where/how is X implemented (huge file) | large file reads | answer contains expected symbol/lines |
| 6 | Find root cause in a 5k-line log with one planted error | big log | answer names planted cause |
| 7 | Git history question (who/when changed X) | git log/diff | exact commit hash |
| 8 | Small-output control task (edit one config value) | none | file check |

Task 8 is a control: it shows what each tool costs when there is nothing to compress.
5 runs per task per arm = 160 runs. Randomised, interleaved order.

Measured per run: pass/fail, tokens by type (input / cache write / cache read / output),
list-price cost, turns, tool calls, wall time. Paired bootstrap 95% intervals vs baseline.

## Risks and how I handle them

- **Subscription limits.** 160 runs may exceed a 5-hour window. The runner pauses and
  resumes; each run is independent, so a pause does not bias results.
- **Proxy arms with subscription auth.** Both proxies forward the OAuth header; this is
  unverified until the first real call. A pilot of 1 run per arm checks it first.
- **Cost isn't a real bill** on a subscription: list-price computed from tokens, stated as such.
- **Docker Hub / GitHub assets are flaky from this machine.** One build workaround
  (`--add-host release-assets.githubusercontent.com:185.199.108.133`) was needed for
  caveman's signed binary download. It does not change what is installed.
- **Authorship bias.** Mitigations are in the protocol. The blog states the conflict up front.

## Decisions I need from you

1. Approve (or edit) the 8-task list and the 5-repeat count.
2. Agent model: `claude-sonnet-5-5` (cheaper on your plan) or `claude-opus-5-5`?
3. Run `claude setup-token` yourself and give me the token as env var `CLAUDE_CODE_OAUTH_TOKEN`
   (never committed). I can't do this interactively for you.
4. OK to run a 4-run pilot (1 trivial task, 1 per arm) to confirm each arm works with
   subscription auth, before the full run?
5. Publish: blog as `bench/BLOG.md` in this repo via a PR from `bench/comparison`
   (main is protected, so you merge it).

## Next steps once approved

1. Pin repos by SHA (`bench/layer2/repos.lock`), write tasks + graders, freeze them.
2. Build Layer 1 corpus + scorer, run it (no cost), review results with you.
3. Pilot, then the full Layer 2 run.
4. Analyse, write the blog with the raw data in `bench/results/`.
