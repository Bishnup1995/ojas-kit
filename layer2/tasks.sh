# Task definitions. Sourced inside the container. Each task has: setup_<id>, prompt_<id>, grade_<id>.
# Graders run AFTER the agent, outside its view; they restore test files from git first so tests cannot be edited to pass.
W=/home/bench/work
TASKS_STRESS="s1_click_full_suite s2_express_full_suite s3_click_core_full_read s4_log_cat_all"
TASKS="t1_click_bug t2_express_bug t3_walkdir_bug t4_click_rename t5_click_qa t6_log_rootcause t7_git_history t8_control_edit"
repo_of() { case "$1" in t1*|t4*|t5*|t7*|s1*|s3*) echo $W/click;; t2*|t8*|s2*) echo $W/express;; t3*) echo $W/walkdir;; t6*|s4*) echo $W/logs;; esac; }
commit_wip() { git add -A && git -c user.name=b -c user.email=b@b commit -qm "$1"; }
# Bug tasks: fold the mutation into the tip commit and drop the old objects, so the agent cannot read the bug off `git diff`/`git show`.
hide_mutation() { git add -A && git -c user.name=b -c user.email=b@b commit -q --amend --no-edit && git reflog expire --expire=now --all && git gc -q --prune=now; }

setup_t1_click_bug()    { cd $W/click && sed -i "s/max(widths.get(idx, 0)/min(widths.get(idx, 0)/" src/click/formatting.py && hide_mutation; }
setup_t2_express_bug()  { cd $W/express && sed -i "0,/this.set('Content-Length', len)/s//this.set('Content-Length', len + 1)/" lib/response.js && hide_mutation; }
setup_t3_walkdir_bug()  { cd $W/walkdir && sed -i "175,177s/self.depth\$/self.depth + 1/" src/dent.rs && hide_mutation; }
setup_t4_click_rename() { :; }
setup_t5_click_qa()     { :; }
setup_t6_log_rootcause(){ mkdir -p $W/logs && python3 /bench/make_log.py; }
setup_t7_git_history()  { cd $W/click && sed -i '0,/^import/s//import  /' src/click/formatting.py && commit_wip "wip: touch formatting"; }
setup_t8_control_edit() { :; }

prompt_t1_click_bug()    { echo "In this repo (the Python library 'click'), several tests fail because of a bug somewhere in the library source under src/. Find the bug and fix it in the source. Do not modify any tests. Ignore failures in tests/test_utils (known, unrelated environment issues). When done, reply with one sentence describing the fix."; }
prompt_t2_express_bug()  { echo "In this repo (the Node.js framework 'express'), many tests fail because of a bug somewhere in lib/. Find the bug and fix it in the library source. Do not modify any tests. When done, reply with one sentence describing the fix."; }
prompt_t3_walkdir_bug()  { echo "In this repo (the Rust crate 'walkdir'), tests fail because of a bug in the library source under src/. Find the bug and fix it in the source. Do not modify any tests. When done, reply with one sentence describing the fix."; }
prompt_t4_click_rename() { echo "In this repo (the Python library 'click'), rename the function make_str to ensure_str everywhere it is defined or used (source, tests, docs, examples). The word make_str must not appear anywhere afterwards except in CHANGES.md. Keep behavior identical. When done, reply 'done'."; }
prompt_t5_click_qa()     { echo "In this repo (the Python library 'click'): in which file, and inside which method of which class, does click catch an Abort exception and print 'Aborted!'? Answer with the file path, class name and method name only. Do not modify any files."; }
prompt_t6_log_rootcause(){ echo "The file /home/bench/work/logs/app.log is an application log from an outage. Find the root cause of the outage and report the incident id (format INC-####). Answer in two sentences. Do not modify any files."; }
prompt_t7_git_history()  { echo "In this repo, which commit most recently modified src/click/formatting.py, ignoring commits whose message starts with 'wip'? Give the full 40-character commit hash and its subject line. Do not modify any files."; }
prompt_t8_control_edit() { echo "In this repo, in examples/hello-world/index.js change the listening port from 3000 to 3001. Do not change anything else. When done, reply 'done'."; }

# ANSWER = the agent's final text. Each grader prints PASS or "FAIL <reason>".
grade_t1_click_bug()    { cd $W/click && git checkout -q -- tests && python -m pytest -q -p no:cacheprovider --ignore=tests/test_utils tests >/tmp/g.log 2>&1 && echo PASS || echo "FAIL pytest: $(tail -1 /tmp/g.log)"; }
grade_t2_express_bug()  { cd $W/express && git checkout -q -- test && npm test >/tmp/g.log 2>&1 && echo PASS || echo "FAIL npm test: $(grep -E 'passing|failing' /tmp/g.log | tr '\n' ' ')"; }
grade_t3_walkdir_bug()  { cd $W/walkdir && git checkout -q -- src/tests && cargo test >/tmp/g.log 2>&1 && echo PASS || echo "FAIL cargo test: $(grep 'test result' /tmp/g.log | head -2 | tr '\n' ' ')"; }
grade_t4_click_rename() {
  cd $W/click
  left=$(grep -rIw make_str . --exclude-dir=.git --exclude=CHANGES.md | wc -l)
  [ "$left" = 0 ] || { echo "FAIL make_str still in $left places"; return; }
  grep -rIqw ensure_str src || { echo "FAIL ensure_str not defined"; return; }
  python -m pytest -q -p no:cacheprovider --ignore=tests/test_utils/test_echo_via_pager.py tests >/tmp/g.log 2>&1 && echo PASS || echo "FAIL pytest: $(tail -1 /tmp/g.log)"
}
grade_t5_click_qa() {
  if echo "$ANSWER" | grep -q "core.py" && echo "$ANSWER" | grep -qw "main" && echo "$ANSWER" | grep -qE "BaseCommand|Command"; then echo PASS; else echo "FAIL answer: $ANSWER"; fi
}
grade_t6_log_rootcause() {
  if echo "$ANSWER" | grep -q "INC-7741" && echo "$ANSWER" | grep -qiE "pool|connection"; then echo PASS; else echo "FAIL answer: $ANSWER"; fi
}
grade_t7_git_history() {
  cd $W/click; exp=$(git log --format=%H --invert-grep --grep='^wip' -1 -- src/click/formatting.py)
  if echo "$ANSWER" | grep -q "$exp"; then echo PASS; else echo "FAIL expected $exp"; fi
}
grade_t8_control_edit() {
  cd $W/express
  f=examples/hello-world/index.js
  # the listening port is what matters; the console.log text may or may not be updated
  if grep -q "app.listen(3001)" $f && ! grep -q "app.listen(3000)" $f && [ "$(git diff --name-only | tr -d '
')" = "$f" ]; then echo PASS; else echo "FAIL edit check"; fi
}

# ---- Stress set (post-hoc, pre-registered in PROTOCOL before running): prompts deliberately force large unfiltered output.
setup_s1_click_full_suite() { cd $W/click && for tf in test_basic test_options test_arguments test_commands test_context test_parser; do sed -i '0,/^    assert /s//    assert not /' tests/$tf.py; done; }
setup_s2_express_full_suite() { cd $W/express && sed -i "0,/\.expect(200/s//.expect(418/" test/res.send.js; }
setup_s3_click_core_full_read() { :; }
setup_s4_log_cat_all() { mkdir -p $W/logs && LOGLINES=20000 python3 /bench/make_log.py; }
prompt_s1_click_full_suite() { echo "In this repo (the Python library 'click'), run the complete test suite once with plain 'python -m pytest' (no filtering, no truncation, no piping, no -q). Then report the name of every test file that contains failing tests. Do not modify any files."; }
prompt_s2_express_full_suite() { echo "In this repo (the Node.js framework 'express'), run the complete test suite once with plain 'npm test' (no filtering, no truncation, no piping). Then report which test fails, the expected and the actual value in its failure message. Do not modify any files."; }
prompt_s3_click_core_full_read() { echo "In this repo (the Python library 'click'), read the whole file src/click/core.py with a single 'cat src/click/core.py' (do not use grep, head, tail or line ranges). Then answer: in Parameter.type_cast_value, which exact error messages can be raised, and under which conditions? Do not modify any files."; }
prompt_s4_log_cat_all() { echo "The file /home/bench/work/logs/app.log is an application log from an outage. Print the whole file with a single 'cat /home/bench/work/logs/app.log' (no grep, head or tail), read it, then give the root cause and the incident id (format INC-####). Answer in two sentences. Do not modify any files."; }
grade_s1_click_full_suite() {
  cd $W/click; files=$(python -m pytest -q -p no:cacheprovider tests 2>&1 | grep '^FAILED' | sed 's/^FAILED //; s/::.*//' | sort -u)
  for f in $files; do b=$(basename $f); echo "$ANSWER" | grep -q "$b" || { echo "FAIL missing $b"; return; }; done; echo PASS
}
grade_s2_express_full_suite() {
  if echo "$ANSWER" | grep -q "418" && echo "$ANSWER" | grep -q "200"; then echo PASS; else echo "FAIL answer: $ANSWER"; fi
}
grade_s3_click_core_full_read() {
  if echo "$ANSWER" | grep -q "Value must be an iterable" && echo "$ANSWER" | grep -qE "Takes \{?nargs\}? values but"; then echo PASS; else echo "FAIL answer: $ANSWER"; fi
}
grade_s4_log_cat_all() { grade_t6_log_rootcause; }
