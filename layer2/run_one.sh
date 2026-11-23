#!/usr/bin/env bash
# Runs INSIDE a fresh arm container: one (task, arm) run. Usage: run_one.sh <arm> <task>
# Needs CLAUDE_CODE_OAUTH_TOKEN and MODEL in the environment. Writes /out/{run.json,grade.txt,tool_stats.txt,transcript/}.
set -uo pipefail
arm=$1; t=$2; source /tasks.sh
setup_$t >/dev/null 2>&1
cd "$(repo_of $t)"
prompt=$(prompt_$t)
case $arm in
  baseline|ojas) CMD=(claude) ;;
  caveman) caveman doctor claude --fix >/out/caveman_start.log 2>&1; sleep 3; CMD=(claude) ;;
  headroom) CMD=(headroom wrap claude --) ;;
esac
start=$(date +%s)
timeout 1800 "${CMD[@]}" -p "$prompt" --model "$MODEL" --output-format json \
  --dangerously-skip-permissions --max-turns 60 >/out/run.raw 2>/out/stderr.txt
echo "exit=$? wall_s=$(( $(date +%s) - start ))" > /out/meta.txt
# headroom wrap prints a banner on stdout before the JSON result; keep the raw file and extract the JSON line for every arm alike
grep "^{" /out/run.raw | tail -1 > /out/run.json
ANSWER=$(jq -r '.result // ""' /out/run.json 2>/dev/null)
grade_$t > /out/grade.txt 2>&1
# evidence that the tool was actually active (best effort, never affects the run)
{ case $arm in
    ojas) ojas gain 2>&1; ojas audit --days 1 2>&1 | head -20 ;;
    caveman) caveman status 2>&1; caveman stats 2>&1 | head -30 ;;
    headroom) headroom perf 2>&1 | head -30 ;;
  esac; } > /out/tool_stats.txt 2>&1
cp -r ~/.claude/projects /out/transcript 2>/dev/null
exit 0
