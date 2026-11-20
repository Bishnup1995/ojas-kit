#!/usr/bin/env bash
# Runs inside the ojas image. Kind per item mirrors what ojas's own command classifier picks for the original command.
kind_of() { case "$1" in
  py_pytest*|js_npm_test*|rs_cargo_test*) echo test;;
  *grep*) echo search;;
  *_find*|*_ls_lR|*_tree) echo listing;;
  git_log*) echo git-log;;
  git_show|git_diff*) echo git-diff;;
  git_status*) echo git-status;;
  *pip_install|*npm_install|*cargo_build|*cargo_clippy) echo build;;
  *) echo generic;;
esac; }
for f in /in/*.txt; do n=$(basename "$f" .txt)
  case "$n" in
    py_cat_*) cp "$f" /tmp/$n.py; ojas read /tmp/$n.py > /out/$n.txt 2>&1 </dev/null;;
    js_cat_*) cp "$f" /tmp/$n.js; ojas read /tmp/$n.js > /out/$n.txt 2>&1 </dev/null;;
    rs_cat_*) cp "$f" /tmp/$n.rs; ojas read /tmp/$n.rs > /out/$n.txt 2>&1 </dev/null;;
    *) ojas filter --kind "$(kind_of "$n")" < "$f" > /out/$n.txt 2>/dev/null;;
  esac
done
