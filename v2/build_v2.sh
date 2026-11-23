#!/usr/bin/env bash
# Build the two new v2 arm images. baseline:2 and headroom:2 are the unchanged v1 images (bench/docker/build.sh).
# ojas3: source exported from the ojas repo's main branch with `git archive` (never the working tree).
# rtk: release binary; --add-host is the same GitHub release-assets workaround documented in PLAN.md.
set -euo pipefail; cd "$(dirname "$0")"; export MSYS_NO_PATHCONV=1
OJAS_REPO=${OJAS_REPO:-/d/token-saver}
rm -rf build/ojas-src && mkdir -p build/ojas-src
git -C "$OJAS_REPO" archive main | tar -x -C build/ojas-src
git -C "$OJAS_REPO" rev-parse main > build/ojas-src/OJAS_COMMIT
docker build -f docker/Dockerfile.ojas3 -t ojasbench/ojas3:2 build
docker build --add-host release-assets.githubusercontent.com:185.199.108.133 -f docker/Dockerfile.rtk -t ojasbench/rtk:2 build
docker images --digests 'ojasbench/*'
