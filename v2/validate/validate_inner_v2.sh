# Runs in a FRESH container, as user bench, with V2 pointing at a copy of bench/v2 (+ layer2/).
# Usage: validate_inner_v2.sh <task> <start|good|bad>. start and bad must FAIL, good must PASS.
source $V2/tasks_v2.sh; t=$1; v=$2
setup_$t >/dev/null 2>&1
ANSWER=""
S=$V2/validate
if [ "$v" = good ]; then case $t in
  l6*) { ( cd $W/click; sed -i 's/min(widths.get(idx, 0)/max(widths.get(idx, 0)/' src/click/formatting.py
         sed -i 's/possible_names.sort(key=lambda x: len(x\[0\]))/possible_names.sort(key=lambda x: -len(x[0]))/' src/click/core.py
         cat $S/duration_impl.py >> src/click/types.py
         sed -i 's/^from .types import DateTime as DateTime$/&\nfrom .types import Duration as Duration/' src/click/__init__.py
         printf 'import click\n\n\ndef test_duration():\n    assert click.Duration().convert("1h", None, None).total_seconds() == 3600\n' > tests/test_types/test_Duration.py
         printf '\n## Duration\n\n`click.Duration` parses `1h30m` into a timedelta.\n' >> docs/parameter-types.md
         sed -i 's/^\.\. autoclass:: Choice$/.. autoclass:: Duration\n   :members:\n\n&/' docs/api.md
         sed -i '0,/^Unreleased$/s//Unreleased\n\n- Add `click.Duration` parameter type./' CHANGES.md
         python3 $S/split_core.py good )
         ( cd $W/express; sed -i "s/? hostname.split('.')\$/? hostname.split('.').reverse()/" lib/request.js
         sed -i "0,/      fn = exports.etag;/s//      fn = exports.wetag;/" lib/utils.js; sed -i "s/  var status = 303;/  var status = 302;/" lib/response.js; sed -i "s/len + 1)/len)/" lib/response.js )
         ( cd $W/walkdir; sed -i '175,177s/self.depth + 1/self.depth/' src/dent.rs ); } ;;
  l4*) ( cd $W/click; sed -i 's/min(widths.get(idx, 0)/max(widths.get(idx, 0)/' src/click/formatting.py
         sed -i 's/possible_names.sort(key=lambda x: len(x\[0\]))/possible_names.sort(key=lambda x: -len(x[0]))/' src/click/core.py
         cat $S/duration_impl.py >> src/click/types.py
         sed -i 's/^from .types import DateTime as DateTime$/&\nfrom .types import Duration as Duration/' src/click/__init__.py
         printf 'import click\n\n\ndef test_duration():\n    assert click.Duration().convert("1h", None, None).total_seconds() == 3600\n' > tests/test_types/test_Duration.py
         printf '\n## Duration\n\n`click.Duration` parses `1h30m` into a timedelta.\n' >> docs/parameter-types.md
         sed -i 's/^\.\. autoclass:: Choice$/.. autoclass:: Duration\n   :members:\n\n&/' docs/api.md
         sed -i '0,/^Unreleased$/s//Unreleased\n\n- Add `click.Duration` parameter type./' CHANGES.md
         python3 $S/split_core.py good ) ;;
  l5*) ( cd $W/express; sed -i "s/? hostname.split('.')\$/? hostname.split('.').reverse()/" lib/request.js
         sed -i "0,/      fn = exports.etag;/s//      fn = exports.wetag;/" lib/utils.js; sed -i "s/  var status = 303;/  var status = 302;/" lib/response.js; sed -i "s/len + 1)/len)/" lib/response.js ) ;;
  r1*) sed -i 's/    if qty > 100:/    if qty >= 100:/' $W/pricing/pricing.py ;;
  r2*) ANSWER=$(python3 $V2/fixtures.py r2-expected | sed 's/^/- `/; s/$/`/') ;;
  r3*) ( cd $W/click; sed -i 's/possible_names.sort(key=lambda x: len(x\[0\]))/possible_names.sort(key=lambda x: -len(x[0]))/' src/click/core.py ) ;;
  l1*) ( cd $W/click; cat $S/duration_impl.py >> src/click/types.py
         sed -i 's/^from .types import DateTime as DateTime$/&\nfrom .types import Duration as Duration/' src/click/__init__.py
         printf 'import click\n\n\ndef test_duration():\n    assert click.Duration().convert("1h", None, None).total_seconds() == 3600\n' > tests/test_types/test_Duration.py
         printf '\n## Duration\n\n`click.Duration` parses `1h30m` into a timedelta.\n' >> docs/parameter-types.md
         sed -i 's/^\.\. autoclass:: Choice$/.. autoclass:: Duration\n   :members:\n\n&/' docs/api.md
         sed -i '0,/^Unreleased$/s//Unreleased\n\n- Add `click.Duration` parameter type./' CHANGES.md ) ;;
  l2*) ( cd $W/express; sed -i "s/? hostname.split('.')\$/? hostname.split('.').reverse()/" lib/request.js
         sed -i "0,/      fn = exports.etag;/s//      fn = exports.wetag;/" lib/utils.js; sed -i "s/  var status = 303;/  var status = 302;/" lib/response.js ) ;;
  l3*) ( cd $W/click; python3 $S/split_core.py good ) ;;
  t1*) ( cd $W/click; sed -i 's/min(widths.get(idx, 0)/max(widths.get(idx, 0)/' src/click/formatting.py ) ;;
  t2*) ( cd $W/express; sed -i "s/len + 1)/len)/" lib/response.js ) ;;
  t3*) ( cd $W/walkdir; sed -i '175,177s/self.depth + 1/self.depth/' src/dent.rs ) ;;
  t4*) ( cd $W/click; grep -rIlw make_str . --exclude-dir=.git --exclude=CHANGES.md | xargs sed -i 's/\bmake_str\b/ensure_str/g' ) ;;
  t5*) ANSWER='src/click/core.py, class Command, method main' ;;
  t6*|s4*) ANSWER='Root cause: db connection pool exhausted. INC-7741' ;;
  t7*) ANSWER=$(cd $W/click; git log --format=%H --invert-grep --grep='^wip' -1 -- src/click/formatting.py) ;;
  t8*) ( cd $W/express; sed -i 's/app.listen(3000)/app.listen(3001)/' examples/hello-world/index.js ) ;;
  s1*) ANSWER=$(cd $W/click; python -m pytest -q -p no:cacheprovider tests 2>&1 | grep '^FAILED' | sed 's/^FAILED //; s/::.*//' | sort -u | tr '\n' ' ') ;;
  s2*) ANSWER='test fails: expected 418 "I am a Teapot", got 200 OK' ;;
  s3*) ANSWER='"Value must be an iterable." and "Takes {nargs} values but {len} were given."' ;;
esac; fi
if [ "$v" = bad ]; then case $t in
  l6*) { ( cd $W/click; sed -i 's/min(widths.get(idx, 0)/max(widths.get(idx, 0)/' src/click/formatting.py
         sed -i 's/possible_names.sort(key=lambda x: len(x\[0\]))/possible_names.sort(key=lambda x: -len(x[0]))/' src/click/core.py
         cat $S/duration_impl.py >> src/click/types.py
         sed -i 's/^from .types import DateTime as DateTime$/&\nfrom .types import Duration as Duration/' src/click/__init__.py
         printf 'import click\n\n\ndef test_duration():\n    assert click.Duration().convert("1h", None, None).total_seconds() == 3600\n' > tests/test_types/test_Duration.py
         printf '\n## Duration\n\n`click.Duration` parses `1h30m` into a timedelta.\n' >> docs/parameter-types.md
         sed -i 's/^\.\. autoclass:: Choice$/.. autoclass:: Duration\n   :members:\n\n&/' docs/api.md
         sed -i '0,/^Unreleased$/s//Unreleased\n\n- Add `click.Duration` parameter type./' CHANGES.md
         python3 $S/split_core.py good )
         ( cd $W/express; sed -i "s/? hostname.split('.')\$/? hostname.split('.').reverse()/" lib/request.js
         sed -i "0,/      fn = exports.etag;/s//      fn = exports.wetag;/" lib/utils.js; sed -i "s/  var status = 303;/  var status = 302;/" lib/response.js; sed -i "s/len + 1)/len)/" lib/response.js ); } ;;   # walkdir bug left in
  l4*) ( cd $W/click; sed -i 's/min(widths.get(idx, 0)/max(widths.get(idx, 0)/' src/click/formatting.py   # second bug (core.py) left in
         cat $S/duration_impl.py >> src/click/types.py
         sed -i 's/^from .types import DateTime as DateTime$/&\nfrom .types import Duration as Duration/' src/click/__init__.py
         printf 'import click\n\n\ndef test_duration():\n    assert click.Duration().convert("1h", None, None).total_seconds() == 3600\n' > tests/test_types/test_Duration.py
         printf '\n## Duration\n\n`click.Duration` parses `1h30m` into a timedelta.\n' >> docs/parameter-types.md
         sed -i 's/^\.\. autoclass:: Choice$/.. autoclass:: Duration\n   :members:\n\n&/' docs/api.md
         sed -i '0,/^Unreleased$/s//Unreleased\n\n- Add `click.Duration` parameter type./' CHANGES.md
         python3 $S/split_core.py good ) ;;
  l5*) ( cd $W/express; sed -i "s/? hostname.split('.')\$/? hostname.split('.').reverse()/" lib/request.js
         sed -i "0,/      fn = exports.etag;/s//      fn = exports.wetag;/" lib/utils.js; sed -i "s/  var status = 303;/  var status = 302;/" lib/response.js ) ;;                                                             # 3 of 4 bugs
  r1*) sed -i 's/    if qty > 100:/    if qty >= 101:/' $W/pricing/pricing.py ;;          # wrong boundary "fix"
  r2*) ANSWER=$( (python3 $V2/fixtures.py r2-expected | tail -n +2; python3 -c "import sys; sys.path.insert(0, '$V2'); import fixtures; print(fixtures.r2_model()['C'][0])") | sort ) ;;  # one missing, one catcher added
  r3*) ( cd $W/click; sed -i 's/possible_names.sort(key=lambda x: len(x\[0\]))/possible_names.sort(key=lambda x: len(x[1]))/' src/click/core.py ) ;;
  l1*) ( cd $W/click; sed '/if (self.min is not None/,/is not in the range/d' $S/duration_impl.py >> src/click/types.py   # no min/max support
         sed -i 's/^from .types import DateTime as DateTime$/&\nfrom .types import Duration as Duration/' src/click/__init__.py
         printf 'import click\n\n\ndef test_duration():\n    assert click.Duration().convert("1h", None, None).total_seconds() == 3600\n' > tests/test_types/test_Duration.py
         printf '\n## Duration\n\nx\n' >> docs/parameter-types.md; sed -i 's/^\.\. autoclass:: Choice$/.. autoclass:: Duration\n\n&/' docs/api.md
         sed -i '0,/^Unreleased$/s//Unreleased\n\n- Add `click.Duration` parameter type./' CHANGES.md ) ;;
  l2*) ( cd $W/express; sed -i "s/? hostname.split('.')\$/? hostname.split('.').reverse()/" lib/request.js; sed -i "s/  var status = 303;/  var status = 302;/" lib/response.js ) ;;  # 2 of 3 bugs
  l3*) ( cd $W/click; python3 $S/split_core.py bad ) ;;                                     # no re-export from core
  *) : ;;  # old tasks: bad = start state
esac; fi
echo "$t $v: $(ANSWER=$ANSWER grade_$t 2>&1 | tail -1 | cut -c1-200)"
