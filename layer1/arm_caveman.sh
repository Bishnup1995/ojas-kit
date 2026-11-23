#!/usr/bin/env bash
# Runs inside the caveman image: the engine's own compress on stdin (stdout=text, stderr=stats).
for f in /in/*.txt; do n=$(basename "$f" .txt)
  ~/.caveman/bin/caveman-engine compress < "$f" > /out/$n.txt 2>/out/$n.meta
done
