#!/usr/bin/env bash
# Build all arm images. Usage: bench/docker/build.sh [arm ...]
set -euo pipefail
cd "$(dirname "$0")"
arms=("${@:-baseline ojas caveman headroom}")
[ $# -eq 0 ] && arms=(baseline ojas caveman headroom)
for a in "${arms[@]}"; do
  docker build --target "$a" -t "ojasbench/$a:2" . 
done
docker images --digests 'ojasbench/*'
