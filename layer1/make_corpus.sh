#!/usr/bin/env bash
# Records raw tool outputs from the pinned repos inside the shared image. Free, no model calls.
# Output: bench/layer1/corpus/<name>.txt  (stdout+stderr, exactly as a shell would show it) + corpus/<name>.cmd
set -uo pipefail
cd "$(dirname "$0")"; mkdir -p corpus
IMG=${IMG:-ojasbench/baseline:2}
MSYS_NO_PATHCONV=1 HERE="$(pwd -W 2>/dev/null || pwd)"
MSYS_NO_PATHCONV=1 docker run --rm -v "$HERE/corpus:/out" -v "$HERE/corpus_inner.sh:/inner.sh:ro" "$IMG" bash /inner.sh </dev/null
wc -c corpus/*.txt | sort -n | tail -50
