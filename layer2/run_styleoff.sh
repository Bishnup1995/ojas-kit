#!/usr/bin/env bash
# Follow-up experiment: does ojas's SessionStart style text cause its main-set overhead?
# Arms: baseline (rerun), ojas (as originally configured, style "full"), ojas-off (same image + {"style":"off"}).
# Main 8 tasks only, 3 reps, interleaved random order. Writes ONLY to ../results/layer2_styleoff/.
# run.sh / run_one.sh / the arm images are untouched; ojas-off uses image ojasbench/ojas-off:2
# (docker/Dockerfile.ojas-off) and is driven by run_one.sh exactly like the ojas arm.
# Usage: bench/layer2/run_styleoff.sh [--reps N] [--seed S] [--budget USD]
set -uo pipefail; cd "$(dirname "$0")"; export MSYS_NO_PATHCONV=1
MODEL=${MODEL:-claude-sonnet-5-5}; REPS=3; SEED=20261121; BUDGET=10
while [ $# -gt 0 ]; do case $1 in --reps) REPS=$2; shift;; --seed) SEED=$2; shift;; --budget) BUDGET=$2; shift;; esac; shift; done
[ -z "${CLAUDE_CODE_OAUTH_TOKEN:-}" ] && [ -f ../../.oauth-token ] && export CLAUDE_CODE_OAUTH_TOKEN="$(cat ../../.oauth-token)"
unset ANTHROPIC_API_KEY ANTHROPIC_AUTH_TOKEN
: "${CLAUDE_CODE_OAUTH_TOKEN:?set CLAUDE_CODE_OAUTH_TOKEN (from: claude setup-token)}"
OUT=../results/layer2_styleoff; RAW=$OUT/raw; mkdir -p "$RAW"
HERE="$(pwd -W 2>/dev/null || pwd)"; RAWABS="$(cd "$RAW" && (pwd -W 2>/dev/null || pwd))"
TASKS="t1_click_bug t2_express_bug t3_walkdir_bug t4_click_rename t5_click_qa t6_log_rootcause t7_git_history t8_control_edit"
SCHEDULE=$(python - "$SEED" "$REPS" $TASKS <<'PY'
import random, sys
seed, reps, tasks = int(sys.argv[1]), int(sys.argv[2]), sys.argv[3:]
arms = ["baseline", "ojas", "ojas-off"]
jobs = [(a, t, r) for r in range(1, reps + 1) for t in tasks for a in arms]
random.Random(seed).shuffle(jobs)
for a, t, r in jobs:
    sys.stdout.buffer.write((a + " " + t + " " + str(r) + chr(10)).encode())
PY
)
SCHEDULE=$(echo "$SCHEDULE" | tr -d '\r')
echo "seed=$SEED reps=$REPS model=$MODEL" > $OUT/schedule.txt; echo "$SCHEDULE" >> $OUT/schedule.txt
docker images --digests 'ojasbench/*' > $OUT/images.txt 2>&1
spent() { python - "$RAW" <<'PY'
import glob, json, sys
s = 0.0
for f in glob.glob(sys.argv[1] + "/*/run.json"):
    try: s += json.load(open(f, encoding="utf-8")).get("total_cost_usd", 0) or 0
    except Exception: pass
print("%.4f" % s)
PY
}
echo "$SCHEDULE" | while read -r arm task rep; do
  name="${arm}__${task}__${rep}"; dir="$RAW/$name"
  [ -f "$dir/grade.txt" ] && continue
  s=$(spent); if python -c "import sys; sys.exit(0 if float('$s') >= float('$BUDGET') else 1)"; then echo "BUDGET STOP: spent \$$s >= \$$BUDGET"; break; fi
  case $arm in ojas-off) img=ojasbench/ojas-off:2; inner=ojas;; *) img=ojasbench/$arm:2; inner=$arm;; esac
  for attempt in 1 2 3 4 5 6; do
    rm -rf -- "${RAW:?}/${name:?}"; mkdir -p "$dir"
    echo "[$(date +%H:%M:%S)] $arm $task #$rep (attempt $attempt, spent \$$s)"
    docker run --rm --memory 6g -e CLAUDE_CODE_OAUTH_TOKEN -e MODEL="$MODEL" \
      -v "$HERE/tasks.sh:/tasks.sh:ro" -v "$HERE/make_log.py:/bench/make_log.py:ro" -v "$HERE/run_one.sh:/run_one.sh:ro" \
      -v "$RAWABS/$name:/out" "$img" bash /run_one.sh "$inner" "$task" </dev/null
    if python - "$dir/run.json" <<'PY'
import json, re, sys
try:
    j = json.load(open(sys.argv[1], encoding="utf-8"))
    sys.exit(0 if j.get("is_error") and re.search(r"usage limit|rate limit|limit reached|overloaded|429", str(j.get("result", "")), re.I) else 1)
except Exception:
    sys.exit(1)
PY
    then echo "  throttled; sleeping 15 min"; sleep 900; continue; fi
    break
  done
  # per-run verification of the arm's state, from the transcript (the SessionStart context is recorded there)
  st=$(grep -rl "Response style (token-saver)" "$dir/transcript" 2>/dev/null | wc -l); hk=$(grep -rl "Command output (token-saver)" "$dir/transcript" 2>/dev/null | wc -l)
  echo "style_full_in_transcript=$st hook_note_in_transcript=$hk" > "$dir/verify.txt"
  echo "  -> $(head -1 "$dir/grade.txt" 2>/dev/null)  $(python -c "import json,sys;j=json.load(open(sys.argv[1],encoding='utf-8'));print('cost=%.4f turns=%s'%(j.get('total_cost_usd',0),j.get('num_turns')))" "$dir/run.json" 2>/dev/null)  $(cat "$dir/verify.txt")"
done
echo "TOTAL spent \$$(spent)"
