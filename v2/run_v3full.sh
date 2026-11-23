#!/usr/bin/env bash
# Haiku check of ojas read --full (DEVIATIONS.md, 2026-11-23).
export PATH="/usr/bin:/bin:/c/Program Files/Docker/Docker/resources/bin:$HOME/AppData/Local/Microsoft/WindowsApps:$PATH"
cd /d/token-saver-bench/bench/v2 || exit 2
./run_study.sh ../results/v3-haiku-full
