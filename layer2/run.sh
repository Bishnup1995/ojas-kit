#!/usr/bin/env bash
# Layer 2 orchestrator (host). Resumable: finished runs are skipped. Usage:
#   CLAUDE_CODE_OAUTH_TOKEN=... bench/layer2/run.sh [--pilot] [--reps N] [--seed S]
set -uo pipefail; cd "$(dirname "$0")"; export MSYS_NO_PATHCONV=1
MODEL=${MODEL:-claude-sonnet-5-5}; REPS=5; SEED=20261120; PILOT=0; STRESS=0
while [ $# -gt 0 ]; do case $1 in --pilot) PILOT=1;; --reps) REPS=$2; shift;; --seed) SEED=$2; shift;; --stress) STRESS=1;; esac; shift; done
[ -z "${CLAUDE_CODE_OAUTH_TOKEN:-}" ] && [ -f ../../.oauth-token ] && export CLAUDE_CODE_OAUTH_TOKEN="$(cat ../../.oauth-token)"
unset ANTHROPIC_API_KEY ANTHROPIC_AUTH_TOKEN
: "${CLAUDE_CODE_OAUTH_TOKEN:?set CLAUDE_CODE_OAUTH_TOKEN (from: claude setup-token)}"
HERE="$(pwd -W 2>/dev/null || pwd)"; RAW=../results/layer2/raw; mkdir -p "$RAW"; RAWABS="$(cd "$RAW" && (pwd -W 2>/dev/null || pwd))"
if [ $STRESS = 1 ]; then source ./tasks.sh; TASKS="$TASKS_STRESS"; SEED=$((SEED+1)); elif [ $PILOT = 1 ]; then TASKS="t8_control_edit"; REPS=1; else TASKS="t1_click_bug t2_express_bug t3_walkdir_bug t4_click_rename t5_click_qa t6_log_rootcause t7_git_history t8_control_edit"; fi
# randomised, interleaved schedule (seed recorded)
SCHEDULE=$(python - "$SEED" "$REPS" $TASKS <<'PY'
import random, sys
seed, reps, tasks = int(sys.argv[1]), int(sys.argv[2]), sys.argv[3:]
arms = ["baseline", "ojas", "caveman", "headroom"]
jobs = [(a, t, r) for r in range(1, reps + 1) for t in tasks for a in arms]
random.Random(seed).shuffle(jobs)
for a, t, r in jobs:
    sys.stdout.buffer.write((a + " " + t + " " + str(r) + chr(10)).encode())
PY
)
SCHEDULE=$(echo "$SCHEDULE" | tr -d '')
echo "$SCHEDULE" > ../results/layer2/schedule.txt
echo "$SCHEDULE" | while read -r arm task rep; do
  dir="$RAW/${arm}__${task}__${rep}"
  [ -f "$dir/grade.txt" ] && continue
  for attempt in 1 2 3 4 5 6; do
    rm -rf "$dir"; mkdir -p "$dir"
    echo "[$(date +%H:%M:%S)] $arm $task #$rep (attempt $attempt)"
    docker run --rm --memory 6g -e CLAUDE_CODE_OAUTH_TOKEN -e MODEL="$MODEL" \
      -v "$HERE/tasks.sh:/tasks.sh:ro" -v "$HERE/make_log.py:/bench/make_log.py:ro" -v "$HERE/run_one.sh:/run_one.sh:ro" \
      -v "$RAWABS/${arm}__${task}__${rep}:/out" "ojasbench/$arm:2" bash /run_one.sh "$arm" "$task" </dev/null
    # usage/rate limit: wait and retry the SAME job so no arm loses a run to throttling
    # throttled only if Claude Code itself reports an error result mentioning a limit (is_error=true), never by substring
    # matches on numbers (an earlier pattern matched "429" inside durations and token counts).
    if python - "$dir/run.json" <<'PY'
import json, re, sys
try:
    j = json.load(open(sys.argv[1], encoding="utf-8"))
    sys.exit(0 if j.get("is_error") and re.search(r"usage limit|rate limit|limit reached|overloaded|429", str(j.get("result", "")), re.I) else 1)
except Exception:
    sys.exit(1)
PY
    then
      echo "  throttled; sleeping 15 min"; sleep 900; continue
    fi
    break
  done
  echo "  -> $(head -1 "$dir/grade.txt" 2>/dev/null)  $(python -c "import json,sys;j=json.load(open(sys.argv[1],encoding='utf-8'));print('cost=%.4f turns=%s'%(j.get('total_cost_usd',0),j.get('num_turns')))" "$dir/run.json" 2>/dev/null)"
done
