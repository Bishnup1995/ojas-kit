#!/usr/bin/env bash
# Replays the corpus through each arm in its own container. Free, deterministic, no model calls.
set -uo pipefail; cd "$(dirname "$0")"
HERE="$(pwd -W 2>/dev/null || pwd)"; export MSYS_NO_PATHCONV=1
arms=("$@"); [ $# -eq 0 ] && arms=(baseline ojas caveman headroom)
for arm in "${arms[@]}"; do
  tag=2
  rm -rf "out/$arm"; mkdir -p "out/$arm"
  if [ "$arm" = headroom ]; then cmd='cd /; ~/.local/share/uv/tools/headroom-ai/bin/python /arm.py'; mount="-v $HERE/arm_headroom.py:/arm.py:ro"
  else cmd="bash /arm.sh"; mount="-v $HERE/arm_$arm.sh:/arm.sh:ro"; fi
  docker run --rm -v "$HERE/corpus:/in:ro" -v "$HERE/out/$arm:/out" $mount "ojasbench/$arm:$tag" bash -lc "$cmd" </dev/null
  echo "$arm: $(ls out/$arm/*.txt 2>/dev/null | wc -l) items"
done
