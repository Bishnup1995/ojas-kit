#!/usr/bin/env bash
# Follow-up 3 (FOLLOWUP3_PREREG.md): study A then study B, one container at a time.
export PATH="/usr/bin:/bin:/c/Program Files/Docker/Docker/resources/bin:$HOME/AppData/Local/Microsoft/WindowsApps:$PATH"
cd /d/token-saver-bench/bench/v2 || exit 2
./run_study.sh ../results/v3-long3 && ./run_study.sh ../results/v3-haiku
