#!/usr/bin/env bash
set -u
cd /home/bench/work
rec() { local name=$1 dir=$2; shift 2; printf '%s\n' "(cd $dir && $*)" > /out/$name.cmd; (cd $dir && eval "$*") > /out/$name.txt 2>&1; }
# --- injected failures live in throw-away copies ---
cp -r click click_bad; cp -r express express_bad; cp -r walkdir walkdir_bad
for t in test_basic test_options test_arguments test_commands test_context test_parser; do sed -i '0,/^    assert /s//    assert not /' click_bad/tests/$t.py; done
sed -i '0,/\.expect(200/s//.expect(418/' express_bad/test/res.send.js
f=$(grep -rl "assert_eq!" walkdir_bad/src | head -1); sed -i '0,/assert_eq!/s//assert_ne!/' "$f"
# python
rec py_pytest_pass      click      "python -m pytest"
rec py_pytest_verbose   click      "python -m pytest -v"
rec py_pytest_fail      click_bad  "python -m pytest"
rec py_grep_def         click      "grep -rn 'def ' src"
rec py_grep_import      click      "grep -rn 'import' src tests"
rec py_find             click      "find . -name '*.py' -not -path './.git/*'"
rec py_ls_lR            click      "ls -lR src tests docs"
rec py_tree             click      "tree -I '.git' src tests"
rec py_cat_core         click      "cat src/click/core.py"
rec py_cat_types        click      "cat src/click/types.py"
rec py_pip_list         click      "pip list"
rec py_pip_install      click      "pip install --force-reinstall --no-deps -e ."
# git
rec git_log             click      "git log -n 300"
rec git_log_stat        click      "git log --stat -n 60"
rec git_show            click      "git show b90faad"
rec git_diff_big        click      "git diff HEAD~120 HEAD"
rec git_status_dirty    click_bad  "git status"
rec git_diff_small      click_bad  "git diff"
# javascript
rec js_npm_test_pass    express    "npm test"
rec js_npm_test_fail    express_bad "npm test"
rec js_grep_function    express    "grep -rn 'function' lib test"
rec js_find             express    "find . -path ./node_modules -prune -o -type f -print"
rec js_find_nm          express    "find node_modules -maxdepth 2 -type d"
rec js_npm_ls_all       express    "npm ls --all"
rec js_npm_ls_json      express    "npm ls --all --json"
rec js_cat_response     express    "cat lib/response.js"
rec js_cat_router_test  express    "cat test/res.send.js"
rec js_npm_install      express    "rm -rf node_modules && npm install --no-audit --no-fund"
# rust
rec rs_cargo_test_pass  walkdir    "cargo test"
rec rs_cargo_test_fail  walkdir_bad "cargo test"
rec rs_cargo_build      walkdir    "cargo clean -q && cargo build"
rec rs_cargo_tree       walkdir    "cargo tree"
rec rs_grep_fn          walkdir    "grep -rn 'fn ' src"
rec rs_cat_lib          walkdir    "cat src/lib.rs"
rec rs_cargo_clippy     walkdir    "cargo clippy --all-targets 2>&1 || true"
# planted-needle log (5000 lines, one FATAL) for fidelity
python3 - <<'PY'
import random
random.seed(7)
lines=[]
for i in range(5000):
    lines.append(f"2026-11-20T09:{i//60%60:02d}:{i%60:02d}Z INFO  worker-{random.randint(1,8)} handled request id={random.randint(10**5,10**6)} status=200 took={random.randint(5,90)}ms")
lines[3712]="2026-11-20T09:01:52Z FATAL worker-3 db pool exhausted: cannot acquire connection after 30s (pool=20 in_use=20 waiting=147) code=PLANTED-7741"
open('/out/log_planted.txt','w').write("\n".join(lines)+"\n")
open('/out/log_planted.cmd','w').write("cat app.log\n")
PY
