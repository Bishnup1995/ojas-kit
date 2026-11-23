# ojas benchmark harness

This branch holds everything behind the numbers in the [ojas README](https://github.com/Bishnup1995/ojas-kit#benchmark-v2) and the [full benchmark report](https://claude.ai/artifact/42CU4Ke8QmYoX6W8GBQm4Z): the tasks, graders, pre-registrations, runners, analysis scripts and per-run results. It has no history in common with `main`, and nothing here ships in the crate.

ojas is written by the person who ran these benchmarks. The pre-registrations were committed before the runs they cover, and every change made after a run started is logged in `v2/DEVIATIONS.md`.

## What's here

| Path | What it is |
|---|---|
| `v2/PREREGISTRATION.md` | Benchmark v2 plan: 18 tasks, 4 setups (no tool, ojas, headroom, rtk), 10 runs each, analysis rules |
| `v2/LONG2_PREREG.md`, `v2/FOLLOWUP3_PREREG.md` | Follow-up studies: longer tasks, the on-call-day session and the Haiku run |
| `v2/DEVIATIONS.md` | Every change made after the first scored run, with date and reason |
| `v2/tasks_v2.sh`, `v2/graders/`, `v2/fixtures.py`, `v2/validate/` | Task prompts, pass/fail graders and the checks that each task starts failing and has a passing answer |
| `v2/run_one_v2.sh`, `v2/run_v2.sh`, `v2/run_study.sh`, `v2/runlib.py` | One run in a fresh container; the resumable schedule runners |
| `v2/docker/`, `docker/` | Images for each setup, with pinned versions |
| `v2/analyze_v2.py`, `v2/rerun_share.py` | Analysis: task-balanced cost ratios, stratified bootstrap (5,000 resamples, seed 1), Fisher tests |
| `results/v2/` | Benchmark v2 (720 runs): `summary.md`, `summary_merged.md`, and one folder per run |
| `results/v2-extra/`, `results/v2-long2/`, `results/v3-long3/`, `results/v3-haiku/` | Follow-ups: the v0.3 read fix, longer tasks, the on-call day, Haiku |
| `PROTOCOL.md`, `layer1/`, `layer2/`, `results/layer*` | Benchmark v1 (earlier, smaller; kept for the record) |

Each run folder keeps `run.json` (Claude Code's result event: turns, tokens, cost), `grade.txt`, `verify.txt`, `meta.txt` and the setup's tool stats. Full transcripts and raw stream logs are left out: they are hundreds of MB.

## Rerunning

You need Docker, Git Bash or a Unix shell, Python 3, and a Claude subscription token in `CLAUDE_CODE_OAUTH_TOKEN` (the runners refuse to start if an API key is set, so runs only use subscription usage).

```bash
docker/build.sh               # base images: baseline and headroom
cd v2
./build_v2.sh                 # ojas3 and rtk images (OJAS_REPO points at an ojas checkout)
./run_v2.sh --smoke           # one run per setup, to check the setup works
./run_v2.sh                   # the full pre-registered schedule (resumable; touch ../results/v2/STOP to pause)
python analyze_v2.py ../results/v2/raw
```

The `.vbs` launchers and `run_v3*.sh` were used to start runs as a Windows scheduled task on the original machine; their paths are specific to it.

Results will vary a little between reruns: the model is not deterministic, and Claude Code and the model change over time. The confidence intervals in each `summary.md` show how much.
