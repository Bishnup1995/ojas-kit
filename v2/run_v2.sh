#!/usr/bin/env bash
# Benchmark v2 runner (host, Git Bash). Resumable, single-instance, one container at a time.
#   bench/v2/run_v2.sh            run (or resume) the full pre-registered schedule (720 runs)
#   bench/v2/run_v2.sh --smoke    one r1 run per arm into results/v2/smoke/ (not part of the schedule)
# Stop gracefully: touch bench/results/v2/STOP (the runner exits after the current run).
# Auth: the user's Claude subscription via CLAUDE_CODE_OAUTH_TOKEN (from ../../.oauth-token). Never an API key:
# the runner aborts if ANTHROPIC_API_KEY / ANTHROPIC_AUTH_TOKEN / ANTHROPIC_BASE_URL is set, if an image carries
# one, or if any run reports a non-subscription auth path (init.apiKeySource != "none").
# Rate limits: the attempt is deleted (not counted), the runner sleeps until the reported reset (else 30 min)
# and retries the same run. Every pause is logged to results/v2/pauses.log.
set -uo pipefail
cd "$(dirname "$0")"; export MSYS_NO_PATHCONV=1
export PATH="/usr/bin:/bin:/c/Program Files/Docker/Docker/resources/bin:$HOME/AppData/Local/Microsoft/WindowsApps:$PATH"
MODEL=claude-sonnet-5-5
SMOKE=0; [ "${1:-}" = "--smoke" ] && SMOKE=1
OUT=../results/v2; mkdir -p "$OUT"
RAW=$OUT/raw; [ $SMOKE = 1 ] && RAW=$OUT/smoke
mkdir -p "$RAW"
PROG=$OUT/progress.log; PAUSES=$OUT/pauses.log
CNAME=ojasbench-v2-run
ts() { date '+%Y-%m-%d %H:%M:%S %Z'; }
log() { echo "[$(ts)] $*" | tee -a "$PROG"; }
die() { log "ABORT: $*"; rm -rf "$OUT/.runner.lock"; exit 2; }

# ---- hard auth guards (abort, never continue)
for v in ANTHROPIC_API_KEY ANTHROPIC_AUTH_TOKEN ANTHROPIC_BASE_URL; do
  [ -n "${!v:-}" ] && { echo "[$(ts)] ABORT: $v is set in the environment; refusing to run (subscription only)" | tee -a "$PROG"; exit 2; }
done
[ -z "${CLAUDE_CODE_OAUTH_TOKEN:-}" ] && [ -f ../../.oauth-token ] && CLAUDE_CODE_OAUTH_TOKEN="$(tr -d '\r\n' < ../../.oauth-token)"
[ -n "${CLAUDE_CODE_OAUTH_TOKEN:-}" ] || { echo "no CLAUDE_CODE_OAUTH_TOKEN and no .oauth-token" | tee -a "$PROG"; exit 2; }
export CLAUDE_CODE_OAUTH_TOKEN
IMAGES="ojasbench/baseline:2 ojasbench/ojas3:2 ojasbench/headroom:2 ojasbench/rtk:2"
for img in $IMAGES; do
  docker image inspect "$img" >/dev/null 2>"$OUT/.docker_err" || { echo "[$(ts)] missing image $img: $(head -c 300 "$OUT/.docker_err")" | tee -a "$PROG"; exit 2; }
  docker image inspect --format '{{range .Config.Env}}{{println .}}{{end}}' "$img" | grep -qE '^ANTHROPIC_(API_KEY|AUTH_TOKEN|BASE_URL)=' \
    && { echo "[$(ts)] ABORT: image $img sets an ANTHROPIC_* auth variable" | tee -a "$PROG"; exit 2; }
done

# ---- single instance (the v1 incident: two runners on one schedule)
if ! mkdir "$OUT/.runner.lock" 2>/dev/null; then
  old=$(cat "$OUT/.runner.lock/pid" 2>/dev/null)
  if [ -n "$old" ] && kill -0 "$old" 2>/dev/null; then echo "another runner (pid $old) is active; exiting"; exit 1; fi
  echo "[$(ts)] stale lock (pid ${old:-?}) removed" | tee -a "$PROG"; rm -rf "$OUT/.runner.lock"; mkdir "$OUT/.runner.lock"
fi
echo $$ > "$OUT/.runner.lock/pid"
trap 'rm -rf "$OUT/.runner.lock"; docker rm -f $CNAME >/dev/null 2>&1' EXIT
trap 'log "runner interrupted"; exit 130' INT TERM
rm -f "$OUT/STOP"

HERE="$(pwd -W 2>/dev/null || pwd)"; L2="$(cd ../layer2 && (pwd -W 2>/dev/null || pwd))"
RAWABS="$(cd "$RAW" && (pwd -W 2>/dev/null || pwd))"
if [ $SMOKE = 1 ]; then
  SCHEDULE=$(printf 'smoke %s r1_pricing_policy 1\n' baseline ojas3 headroom rtk)
else
  SCHEDULE=$(python runlib.py schedule | tr -d '\r')
  echo "$SCHEDULE" > "$OUT/schedule.txt"
fi
TOTAL=$(echo "$SCHEDULE" | wc -l)
{ echo "# $(ts) model=$MODEL"; for img in $IMAGES; do docker image inspect --format "$img {{.Id}} created={{.Created}}" "$img"; done
  docker run --rm --entrypoint cat ojasbench/ojas3:2 /usr/local/share/ojas_commit | sed 's/^/ojas_commit=/'
  docker run --rm --entrypoint cat ojasbench/rtk:2 /usr/local/share/rtk_tag | sed 's/^/rtk_tag=/'; } >> "$OUT/images.txt" 2>&1

complete() { [ -s "$1/run.json" ] && [ -s "$1/grade.txt" ] && [ -s "$1/verify.txt" ]; }
pause_until() {  # $1 epoch (0 = unknown), $2 reason, $3 job
  local now=$(date +%s) until=$1 wait
  if [ "$until" -le "$now" ] 2>/dev/null; then wait=1800; until=$((now + wait)); else wait=$((until - now + 180)); until=$((until + 180)); fi
  [ $wait -lt 120 ] && wait=120
  [ $wait -gt 86400 ] && { wait=86400; until=$((now + wait)); }
  echo "[$(ts)] PAUSE $3: $2 -> sleeping ${wait}s until $(date -d @$until '+%Y-%m-%d %H:%M:%S %Z')" | tee -a "$PAUSES" "$PROG"
  echo "$(date -d @$until '+%Y-%m-%d %H:%M:%S %Z')" > "$OUT/.last_pause"
  sleep $wait
  echo "[$(ts)] RESUME $3" | tee -a "$PAUSES" "$PROG"
}

n=0
log "runner start pid=$$ total=$TOTAL raw=$RAW"
while read -r block arm task rep; do
  n=$((n + 1)); name="${arm}__${task}__${rep}"; dir="$RAW/$name"
  complete "$dir" && continue
  [ -f "$OUT/STOP" ] && { log "STOP file found; exiting before $name ($((n-1))/$TOTAL)"; rm -f "$OUT/STOP"; exit 0; }
  transient=0
  while :; do
    [ -f "$OUT/STOP" ] && { log "STOP file found; exiting before $name"; rm -f "$OUT/STOP"; exit 0; }
    rm -rf -- "${RAW:?}/${name:?}"; mkdir -p "$dir"
    docker rm -f $CNAME >/dev/null 2>&1   # never two containers: fixed name, stale one removed
    log "start $n/$TOTAL block=$block $arm $task #$rep"
    docker run --rm --name $CNAME --user root --memory 6g -e CLAUDE_CODE_OAUTH_TOKEN -e MODEL="$MODEL" \
      -v "$HERE:/root/v2:ro" -v "$L2:/root/layer2:ro" -v "$RAWABS/$name:/out" \
      "ojasbench/$arm:2" bash /root/v2/run_one_v2.sh "$arm" "$task" </dev/null >"$dir/docker.log" 2>&1
    c=$(python runlib.py classify "$dir")
    case "$c" in
      OK) break ;;
      ABORT*) die "$name: $c" ;;
      RATELIMIT*) reset=$(echo "$c" | awk '{print $2}'); rm -rf -- "${RAW:?}/${name:?}"
                  pause_until "$reset" "rate limit: ${c#RATELIMIT * }" "$name" ;;
      TRANSIENT*) transient=$((transient + 1)); rm -rf -- "${RAW:?}/${name:?}"
                  [ $transient -ge 6 ] && die "$name: 6 consecutive transient failures, last: $c"
                  echo "[$(ts)] TRANSIENT $name (#$transient): $c -> retry in 5 min" | tee -a "$PAUSES" "$PROG"; sleep 300 ;;
    esac
  done
  vr=$(python runlib.py verify "$dir" "$arm")
  case "$vr" in *"apiKeySource=none"*) ;; *) die "$name: auth check failed: $vr" ;; esac
  done_n=$(for d in "$RAW"/*__*__*; do complete "$d" && echo; done | wc -l)
  log "done $done_n/$TOTAL block=$block $arm $task #$rep -> $(python runlib.py line "$dir") | $(echo "$vr" | grep -o 'valid=[01]') | last pause: $(cat "$OUT/.last_pause" 2>/dev/null || echo none)"
done <<< "$SCHEDULE"
log "runner finished: $(for d in "$RAW"/*__*__*; do complete "$d" && echo; done | wc -l)/$TOTAL complete"
