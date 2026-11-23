#!/usr/bin/env bash
# Runs INSIDE a fresh arm container, as root: one (arm, task) run. Usage: run_one_v2.sh <arm> <task>
# Harness files are mounted read-only under /root (mode 700, so the agent, user bench, cannot read tasks,
# graders or fixtures generators). They are staged to a temp dir only for setup and for grading, never
# while the agent runs. The agent itself runs as user bench with HOME=/home/bench.
# Writes /out/{run.raw,run.json,init.json,stderr.txt,meta.txt,grade.txt,tool_stats.txt,settings.json,transcript/,...}
set -uo pipefail
arm=$1; t=$2
source /root/v2/stage.sh
# ---- auth guard: subscription (OAuth token) only, never an API key
if [ -n "${ANTHROPIC_API_KEY:-}" ] || [ -n "${ANTHROPIC_AUTH_TOKEN:-}" ] || [ -n "${ANTHROPIC_BASE_URL:-}" ]; then
  echo "ANTHROPIC_API_KEY/AUTH_TOKEN/BASE_URL present in container env" > /out/ABORT; exit 3; fi
[ -n "${CLAUDE_CODE_OAUTH_TOKEN:-}" ] || { echo "no CLAUDE_CODE_OAUTH_TOKEN" > /out/ABORT; exit 3; }
[ -f /home/bench/.claude.json ] && grep -q '"primaryApiKey"' /home/bench/.claude.json && { echo "api key stored in image" > /out/ABORT; exit 3; }

# ---- setup (as bench), then remove harness files before the agent starts
stage_in
asbench bash -c "source \$V2/tasks_v2.sh; setup_$t" > /out/setup.log 2>&1
echo "setup_exit=$?" >> /out/setup.log
repo=$(asbench bash -c "source \$V2/tasks_v2.sh; repo_of_v2 $t")
block=$(asbench bash -c "source \$V2/tasks_v2.sh; block_of $t")
prompt=$(asbench bash -c "source \$V2/tasks_v2.sh; prompt_$t")
stage_out
MAXTURNS=60; TO=1800; [ "$block" = long ] && TO=3600
[ "$block" = long2 ] && { MAXTURNS=100; TO=5400; }   # long2 follow-up study only (LONG2_PREREG.md)
[ "$block" = long3 ] && { MAXTURNS=150; TO=7200; }   # long3 on-call day only (FOLLOWUP3_PREREG.md)
case $arm in
  baseline|ojas3|rtk) CMD=(claude) ;;
  headroom) CMD=(headroom wrap claude --) ;;
  *) echo "unknown arm $arm" > /out/ABORT; exit 3 ;;
esac
echo "arm=$arm task=$t block=$block repo=$repo model=$MODEL max_turns=$MAXTURNS timeout_s=$TO" > /out/meta.txt
cd "$repo"
start=$(date +%s)
timeout -k 30 $TO runuser -u bench -- env -i HOME=/home/bench USER=bench LOGNAME=bench SHELL=/bin/bash PATH="$PATH" TERM=dumb LANG=C.UTF-8 \
  DISABLE_AUTOUPDATER=1 DISABLE_TELEMETRY=1 CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC=1 ARM="$ARM" \
  CLAUDE_CODE_OAUTH_TOKEN="$CLAUDE_CODE_OAUTH_TOKEN" \
  "${CMD[@]}" -p "$prompt" --model "$MODEL" --output-format stream-json --verbose \
  --dangerously-skip-permissions --max-turns $MAXTURNS >/out/run.raw 2>/out/stderr.txt </dev/null
rc=$?; wall=$(( $(date +%s) - start ))
echo "exit=$rc wall_s=$wall" >> /out/meta.txt
# stream-json: the init event carries apiKeySource/model/tools; the last line is the result object (same schema as
# --output-format json). headroom wrap prints a banner first; only JSON lines are used, for every arm alike.
# A background task the agent started can end after the task, making Claude Code emit a second result event.
# num_turns, usage and duration_ms are per event; total_cost_usd is cumulative. Merge: sum the former, keep the last.
grep '^{' /out/run.raw | grep '"type":"result"' > /out/results.jsonl
if [ -s /out/results.jsonl ]; then
  jq -cs 'if length == 1 then .[0] else .[-1] + {num_turns: (map(.num_turns // 0) | add),
    duration_ms: (map(.duration_ms // 0) | add), result: (map(.result // "") | join("\n\n")), n_results: length,
    usage: (reduce (.[].usage // {} | to_entries[] | select(.value | type == "number")) as $e ({}; .[$e.key] += $e.value))} end' \
    /out/results.jsonl > /out/run.json
else : > /out/run.json; fi
grep '^{' /out/run.raw | grep -m1 '"subtype":"init"' > /out/init.json
# A run killed by the wall-clock timeout is a scored failure, not a retry: record a synthetic result object.
if [ ! -s /out/run.json ] && [ $rc -eq 124 -o $rc -eq 137 ]; then
  echo "{\"type\":\"result\",\"subtype\":\"timeout\",\"is_error\":true,\"result\":\"\",\"num_turns\":null,\"total_cost_usd\":null,\"duration_ms\":$((wall*1000))}" > /out/run.json
fi
ANSWER=$(jq -r '.result // ""' /out/run.json 2>/dev/null)

# ---- grade (as bench, harness staged back in after the agent has finished)
stage_in
asbench env ANSWER="$ANSWER" bash -c "source \$V2/tasks_v2.sh; grade_$t" > /out/grade.txt 2>&1
stage_out

# ---- evidence that the arm's tool was active (best effort, never affects the run or the grade)
asbench bash -c "case \$ARM in
    ojas3) ojas gain 2>&1; cp ~/.ojas/stats.jsonl /tmp/ojas_stats.jsonl 2>/dev/null ;;
    rtk) rtk gain 2>&1; rtk gain --history 2>&1 | head -60 ;;
    headroom) headroom perf 2>&1 | head -40 ;;
  esac" > /out/tool_stats.txt 2>&1
cp /tmp/ojas_stats.jsonl /out/ojas_stats.jsonl 2>/dev/null
cp /home/bench/.claude/settings.json /out/settings.json 2>/dev/null
cp -r /home/bench/.claude/projects /out/transcript 2>/dev/null
exit 0
