#!/usr/bin/env bash
# Free sanity check, no model calls. For every v2 task: start state must FAIL, known-good solution must PASS,
# known-bad solution must FAIL. Each check runs in a fresh baseline container. Output: ../results/v2/validation.txt
set -uo pipefail; cd "$(dirname "$0")"; export MSYS_NO_PATHCONV=1
H="$(pwd -W 2>/dev/null || pwd)"; L2="$(cd ../layer2 && (pwd -W 2>/dev/null || pwd))"
mkdir -p ../results/v2; out=${VALIDATION_OUT:-../results/v2/validation.txt}; : > $out
TASKS=${*:-"r1_pricing_policy r2_ledger_raises r3_click_core_bug s1_click_full_suite s2_express_full_suite s3_click_core_full_read s4_log_cat_all t1_click_bug t2_express_bug t3_walkdir_bug t4_click_rename t5_click_qa t6_log_rootcause t7_git_history t8_control_edit l1_click_duration l2_express_ci l3_click_split_core"}
for t in $TASKS; do for v in start good bad; do
  case "$t:$v" in [st]*:bad) continue;; esac   # v1 tasks: bad == start state
  docker run --rm --user root -v "$H:/root/v2:ro" -v "$L2:/root/layer2:ro" ojasbench/baseline:2 \
    bash -c "source /root/v2/stage.sh; stage_in; ARM=baseline asbench bash /tmp/v2s/validate/validate_inner_v2.sh $t $v" </dev/null 2>&1 | tail -1 | tee -a $out
done; done
