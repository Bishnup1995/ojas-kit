#!/usr/bin/env bash
# Offline smoke test: proves each arm's tool is installed and can start. Makes NO model calls.
# Usage: bench/smoke.sh [arm ...]
set -uo pipefail
arms=("$@"); [ $# -eq 0 ] && arms=(baseline ojas caveman headroom)
declare -A CHECK
CHECK[baseline]='claude --version; test ! -e ~/.claude/settings.json && echo "no hooks/settings (expected)"'
CHECK[ojas]='ojas --version; grep -c ojas ~/.claude/settings.json | sed "s/^/ojas hook lines: /"; ojas rewrite "cargo test"'
CHECK[caveman]='caveman --version 2>&1 | head -1; (caveman doctor claude --fix >/tmp/doc.log 2>&1; sleep 3; curl -s -m 5 -o /dev/null -w "proxy 8787 http=%{http_code}\n" http://127.0.0.1:8787/ ; caveman status 2>&1 | head -9)'
CHECK[headroom]='headroom --version; (headroom proxy --port 8787 >/tmp/hr.log 2>&1 & for i in $(seq 1 40); do curl -s -m 2 -o /dev/null http://127.0.0.1:8787/health && break; sleep 2; done; curl -s -m 5 http://127.0.0.1:8787/health | head -c 300; echo)'
rc=0
for a in "${arms[@]}"; do
  echo "=== $a"
  docker run --rm "ojasbench/$a:1" bash -lc "${CHECK[$a]}" || rc=1
done
exit $rc
