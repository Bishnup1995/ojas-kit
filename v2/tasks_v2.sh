# v2 task definitions. Sourced inside the container (run_one_v2.sh), with V2 = a copy of bench/v2 that is only
# present during setup and grading (never while the agent runs). Re-uses the v1 tasks (stress s1-s4, main t1-t8)
# from bench/layer2/tasks.sh unchanged, and adds the read (r1-r3) and long (l1-l3) blocks.
source "$V2/layer2/tasks.sh"
TASKS_READ="r1_pricing_policy r2_ledger_raises r3_click_core_bug"
TASKS_LONG="l1_click_duration l2_express_ci l3_click_split_core"
repo_of_v2() { case "$1" in r1*) echo $W/pricing;; r2*) echo $W/ledger;; r3*|l1*|l3*) echo $W/click;; l2*) echo $W/express;; *) repo_of "$1";; esac; }
block_of() { case "$1" in r*) echo read;; s*) echo stress;; t*) echo main;; l*) echo long;; esac; }

# ---- read block
setup_r1_pricing_policy() { python3 $V2/fixtures.py r1 $W/pricing; }
setup_r2_ledger_raises()  { python3 $V2/fixtures.py r2 $W/ledger; }
setup_r3_click_core_bug() { cd $W/click && sed -i 's/possible_names.sort(key=lambda x: -len(x\[0\]))/possible_names.sort(key=lambda x: len(x[0]))/' src/click/core.py && hide_mutation; }
prompt_r1_pricing_policy() { echo "The directory /home/bench/work/pricing contains pricing.py, a wholesale pricing engine, and README.md, which states the published price policy. Customers report that some quotes do not match the published policy. Find where pricing.py deviates from the policy and fix it so that it follows the policy. Do not change anything else. When done, reply with one sentence describing the fix."; }
prompt_r2_ledger_raises()  { echo "In /home/bench/work/ledger/ledger.py: which module-level functions can raise LedgerError when called, either directly or because they call (directly or indirectly) another function in this file that can, without catching it? Work this out by reading the source; do not execute, import or write scripts to analyse the module. Reply with only the function names, one per line, sorted alphabetically, and nothing else."; }
prompt_r3_click_core_bug() { echo "In this repo (the Python library 'click'), many tests fail because of a bug in src/click/core.py. Find the bug and fix it in that file. Do not modify any tests. Ignore failures in tests/test_utils (known, unrelated environment issues). When done, reply with one sentence describing the fix."; }
grade_r1_pricing_policy() { python3 $V2/fixtures.py r1-ref /tmp/r1ref && python3 $V2/graders/grade_r1.py $W/pricing/pricing.py /tmp/r1ref/pricing.py; }
grade_r2_ledger_raises()  { python3 $V2/fixtures.py r2-expected > /tmp/r2exp && ANSWER="$ANSWER" python3 $V2/graders/grade_r2.py /tmp/r2exp; }
grade_r3_click_core_bug() { grade_t1_click_bug; }

# ---- long block
setup_l1_click_duration() { :; }
setup_l2_express_ci() {
  cd $W/express \
  && sed -i "s/? hostname.split('.').reverse()/? hostname.split('.')/" lib/request.js \
  && sed -i "0,/      fn = exports.wetag;/s//      fn = exports.etag;/" lib/utils.js \
  && sed -i "s/  var status = 302;/  var status = 303;/" lib/response.js \
  && hide_mutation && python3 $V2/graders/make_ci_log.py
}
setup_l3_click_split_core() { :; }
prompt_l1_click_duration() { cat <<'EOF'
In this repo (the Python library 'click'), add a new parameter type click.Duration that parses human-friendly durations into datetime.timedelta. Requirements:
- Implement it in src/click/types.py as a ParamType subclass with name "duration", and export it as click.Duration.
- Accepted input: one or more <integer><unit> parts with units d, h, m, s (case-insensitive), each unit at most once and in that order (d, h, m, s), e.g. "90s", "15m", "1h30m", "1d2h3m4s". A plain non-negative integer such as "45" means seconds. Leading/trailing whitespace is ignored. Anything else (empty, negative, fractional, unknown unit, wrong order, repeated unit, inner spaces) is invalid and must fail with a message containing "is not a valid duration".
- A datetime.timedelta value is passed through unchanged.
- Optional keyword arguments min and max (timedelta or None, inclusive bounds); a value outside them must fail with a message containing "is not in the range".
- Add tests in a new file tests/test_types/test_Duration.py, document the type in docs/parameter-types.md and in the types section of docs/api.md, and add an entry to the unreleased section of CHANGES.md.
- The whole test suite must pass (ignore failures in tests/test_utils, known environment issues).
When done, reply 'done'.
EOF
}
prompt_l2_express_ci() { echo "CI for this repo (the Node.js framework 'express') is red. The complete log of the failing CI job is at /home/bench/work/ci/ci.log. Use the log and the source to find out why, then fix the library code under lib/ so that the whole test suite passes. There may be more than one cause. Do not modify any tests. When done, reply with one line per bug you fixed."; }
prompt_l3_click_split_core() { echo "In this repo (the Python library 'click'), src/click/core.py is too large. Move the classes Parameter, Option and Argument (and any helper used only by them) out of core.py into a new module src/click/params.py. Afterwards these classes must be defined in params.py and no longer in core.py, while 'from click.core import Parameter, Option, Argument', 'from click.params import Parameter, Option, Argument' and click.Option / click.Argument / click.Parameter all keep working and refer to the same classes. Keep behaviour identical and update imports in the rest of the package as appropriate. Do not modify any tests. The whole test suite must pass (ignore failures in tests/test_utils, known environment issues). When done, reply 'done'."; }
grade_l1_click_duration() {
  cd $W/click
  git diff --quiet HEAD -- CHANGES.md && { echo "FAIL CHANGES.md not updated"; return; }
  git diff HEAD -- CHANGES.md | grep -q '^+.*Duration' || { echo "FAIL no Duration entry in CHANGES.md"; return; }
  grep -q Duration docs/parameter-types.md || { echo "FAIL not documented in docs/parameter-types.md"; return; }
  grep -q Duration docs/api.md || { echo "FAIL not in docs/api.md"; return; }
  grep -q Duration tests/test_types/test_Duration.py 2>/dev/null || { echo "FAIL no tests in tests/test_types/test_Duration.py"; return; }
  grep -qE '^class Duration\b' src/click/types.py || { echo "FAIL click.Duration not defined in types.py"; return; }
  cp $V2/graders/test_duration_hidden.py /tmp/test_duration_hidden.py
  python -m pytest -q -p no:cacheprovider /tmp/test_duration_hidden.py >/tmp/h.log 2>&1 || { echo "FAIL hidden tests: $(tail -1 /tmp/h.log)"; return; }
  python -m pytest -q -p no:cacheprovider --ignore=tests/test_utils tests >/tmp/g.log 2>&1 && echo PASS || echo "FAIL pytest: $(tail -1 /tmp/g.log)"
}
grade_l2_express_ci() { grade_t2_express_bug; }
grade_l3_click_split_core() {
  cd $W/click
  [ -f src/click/params.py ] || { echo "FAIL no src/click/params.py"; return; }
  for c in Parameter Option Argument; do
    grep -qE "^class $c\b" src/click/params.py || { echo "FAIL $c not defined in params.py"; return; }
    grep -qE "^class $c\b" src/click/core.py && { echo "FAIL $c still defined in core.py"; return; }
  done
  python -c "
import click, click.core as c, click.params as p
from click.core import Parameter, Option, Argument
assert click.Option is p.Option is c.Option is Option
assert click.Argument is p.Argument is c.Argument is Argument
assert click.Parameter is p.Parameter is c.Parameter is Parameter
assert p.Option.__module__ == 'click.params'
" >/tmp/i.log 2>&1 || { echo "FAIL import check: $(tail -1 /tmp/i.log)"; return; }
  git checkout -q -- tests
  python -m pytest -q -p no:cacheprovider --ignore=tests/test_utils tests >/tmp/g.log 2>&1 && echo PASS || echo "FAIL pytest: $(tail -1 /tmp/g.log)"
}

# ---- long2 block: follow-up long-session study (added 2026-11-23 after the 720 pre-registered runs; see
# LONG2_PREREG.md). Each task chains existing, already-validated tasks into one session so it cannot end in a
# few turns. Everything above this line is unchanged.
TASKS_LONG2="l4_click_sprint l5_express_ci_plus"
repo_of_v2() { case "$1" in r1*) echo $W/pricing;; r2*) echo $W/ledger;; r3*|l1*|l3*|l4*) echo $W/click;; l2*|l5*) echo $W/express;; *) repo_of "$1";; esac; }
block_of() { case "$1" in r*) echo read;; s*) echo stress;; t*) echo main;; l4*|l5*) echo long2;; l*) echo long;; esac; }
setup_l4_click_sprint() { setup_t1_click_bug && setup_r3_click_core_bug; }
prompt_l4_click_sprint() { cat <<'EOT'
You are working in this repo (the Python library 'click'). Do these three jobs in order, in this one session.
1. The test suite is failing. Find and fix the cause in the library source under src/ (there may be more than one bug). Do not modify any tests.
2. Add a new parameter type click.Duration that parses human-friendly durations into datetime.timedelta:
   - Implement it in src/click/types.py as a ParamType subclass with name "duration", and export it as click.Duration.
   - Accepted input: one or more <integer><unit> parts with units d, h, m, s (case-insensitive), each unit at most once and in that order (d, h, m, s), e.g. "90s", "15m", "1h30m", "1d2h3m4s". A plain non-negative integer such as "45" means seconds. Leading/trailing whitespace is ignored. Anything else (empty, negative, fractional, unknown unit, wrong order, repeated unit, inner spaces) is invalid and must fail with a message containing "is not a valid duration".
   - A datetime.timedelta value is passed through unchanged.
   - Optional keyword arguments min and max (timedelta or None, inclusive bounds); a value outside them must fail with a message containing "is not in the range".
   - Add tests in a new file tests/test_types/test_Duration.py, document the type in docs/parameter-types.md and in the types section of docs/api.md, and add an entry to the unreleased section of CHANGES.md.
3. src/click/core.py is too large. Move the classes Parameter, Option and Argument (and any helper used only by them) out of core.py into a new module src/click/params.py. Afterwards these classes must be defined in params.py and no longer in core.py, while 'from click.core import Parameter, Option, Argument', 'from click.params import Parameter, Option, Argument' and click.Option / click.Argument / click.Parameter all keep working and refer to the same classes. Keep behaviour identical and update imports in the rest of the package as appropriate. Do not modify existing tests.
Run the whole test suite after each job, and make sure it passes before moving on (ignore failures in tests/test_utils, known environment issues). When all three jobs are done, reply 'done'.
EOT
}
grade_l4_click_sprint() {
  local a; a=$(grade_l1_click_duration 2>&1 | tail -1)
  case "$a" in PASS) ;; *) echo "FAIL job 1/2: $a"; return;; esac
  grade_l3_click_split_core
}
setup_l5_express_ci_plus() {
  cd $W/express \
  && sed -i "s/? hostname.split('.').reverse()/? hostname.split('.')/" lib/request.js \
  && sed -i "0,/      fn = exports.wetag;/s//      fn = exports.etag;/" lib/utils.js \
  && sed -i "s/  var status = 302;/  var status = 303;/" lib/response.js \
  && sed -i "0,/this.set('Content-Length', len)/s//this.set('Content-Length', len + 1)/" lib/response.js \
  && hide_mutation && python3 $V2/graders/make_ci_log.py
}
prompt_l5_express_ci_plus() { echo "CI for this repo (the Node.js framework 'express') is red. The complete log of the failing CI job is at /home/bench/work/ci/ci.log. Use the log and the source to find out why, then fix the library code under lib/ so that the whole test suite passes. There are several independent causes. Work like you would on a real CI failure: fix one cause at a time, and after each fix run the whole test suite (npm test) again to confirm it and to see what is still failing. Do not modify any tests. When done, reply with one line per bug you fixed."; }
grade_l5_express_ci_plus() { grade_t2_express_bug; }

# ---- long3 block: on-call day (added 2026-11-23 after long2; see FOLLOWUP3_PREREG.md). One session, three repos,
# five jobs, each built from an already-validated task: the l4 click sprint (2 bugs + Duration + split core), the l5
# express CI failure (4 causes) and the t3 walkdir bug. Everything above this line is unchanged.
TASKS_LONG3="l6_oncall_day"
repo_of_v2() { case "$1" in l6*) echo $W;; r1*) echo $W/pricing;; r2*) echo $W/ledger;; r3*|l1*|l3*|l4*) echo $W/click;; l2*|l5*) echo $W/express;; *) repo_of "$1";; esac; }
block_of() { case "$1" in r*) echo read;; s*) echo stress;; t*) echo main;; l6*) echo long3;; l4*|l5*) echo long2;; l*) echo long;; esac; }
setup_l6_oncall_day() { setup_l4_click_sprint && setup_l5_express_ci_plus && setup_t3_walkdir_bug; }
prompt_l6_oncall_day() { cat <<'EOT'
You are on call today for three repos under /home/bench/work: click (Python), express (Node.js) and walkdir (Rust). Work through this queue in order, in this one session. Don't modify existing tests in any repo.

Repo /home/bench/work/click:
1. The test suite is failing. Find and fix the cause in the library source under src/ (there may be more than one bug).
2. Add a new parameter type click.Duration that parses human-friendly durations into datetime.timedelta:
   - Implement it in src/click/types.py as a ParamType subclass with name "duration", and export it as click.Duration.
   - Accepted input: one or more <integer><unit> parts with units d, h, m, s (case-insensitive), each unit at most once and in that order (d, h, m, s), e.g. "90s", "15m", "1h30m", "1d2h3m4s". A plain non-negative integer such as "45" means seconds. Leading/trailing whitespace is ignored. Anything else (empty, negative, fractional, unknown unit, wrong order, repeated unit, inner spaces) is invalid and must fail with a message containing "is not a valid duration".
   - A datetime.timedelta value is passed through unchanged.
   - Optional keyword arguments min and max (timedelta or None, inclusive bounds); a value outside them must fail with a message containing "is not in the range".
   - Add tests in a new file tests/test_types/test_Duration.py, document the type in docs/parameter-types.md and in the types section of docs/api.md, and add an entry to the unreleased section of CHANGES.md.
3. src/click/core.py is too large. Move the classes Parameter, Option and Argument (and any helper used only by them) out of core.py into a new module src/click/params.py. Afterwards these classes must be defined in params.py and no longer in core.py, while 'from click.core import Parameter, Option, Argument', 'from click.params import Parameter, Option, Argument' and click.Option / click.Argument / click.Parameter all keep working and refer to the same classes. Keep behaviour identical and update imports in the rest of the package as appropriate.
For click, run the whole test suite after each job and make sure it passes before moving on (ignore failures in tests/test_utils, known environment issues).

Repo /home/bench/work/express:
4. CI is red. The complete log of the failing CI job is at /home/bench/work/ci/ci.log. Use the log and the source to find out why, then fix the library code under lib/ so that the whole test suite passes. There are several independent causes. Fix one cause at a time, and after each fix run the whole test suite (npm test) again to confirm it and to see what is still failing.

Repo /home/bench/work/walkdir:
5. Tests fail because of a bug in the library source under src/. Find the bug and fix it in the source, then run cargo test.

When the whole queue is done, reply with one line per job.
EOT
}
grade_l6_oncall_day() {
  local a
  a=$(grade_l4_click_sprint 2>&1 | tail -1); case "$a" in PASS) ;; *) echo "FAIL click jobs: $a"; return;; esac
  a=$(grade_l5_express_ci_plus 2>&1 | tail -1); case "$a" in PASS) ;; *) echo "FAIL express job: $a"; return;; esac
  grade_t3_walkdir_bug
}
