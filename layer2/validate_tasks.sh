#!/usr/bin/env bash
# Free sanity check, no model calls. Graders must FAIL at the start state and PASS on a known-good solution.
set -uo pipefail; cd "$(dirname "$0")"; export MSYS_NO_PATHCONV=1
HERE="$(pwd -W 2>/dev/null || pwd)"
for t in t1_click_bug t2_express_bug t3_walkdir_bug t4_click_rename t5_click_qa t6_log_rootcause t7_git_history t8_control_edit s1_click_full_suite s2_express_full_suite s3_click_core_full_read s4_log_cat_all; do
  docker run --rm -v "$HERE:/bench:ro" -v "$HERE/tasks.sh:/tasks.sh:ro" -v "$HERE/validate_inner.sh:/v.sh:ro" ojasbench/baseline:2 bash /v.sh "$t" </dev/null 2>&1 | grep -E "start-state|solved"
done
