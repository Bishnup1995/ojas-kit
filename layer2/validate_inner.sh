# Runs in a FRESH container per task ($1 = task id). Start state must FAIL; known-good solution must PASS.
source /tasks.sh; t=$1
( setup_$t >/dev/null 2>&1; ANSWER=""; echo "$t start-state: $(grade_$t 2>&1 | tail -1 | cut -c1-100)" )
setup_$t >/dev/null 2>&1
case $t in
  t1*) ( cd $W/click; sed -i 's/min(widths.get(idx, 0)/max(widths.get(idx, 0)/' src/click/formatting.py ) ;;
  t2*) ( cd $W/express; sed -i "s/len + 1)/len)/" lib/response.js ) ;;
  t3*) ( cd $W/walkdir; sed -i '175,177s/self.depth + 1/self.depth/' src/dent.rs ) ;;
  t4*) ( cd $W/click; grep -rIlw make_str . --exclude-dir=.git --exclude=CHANGES.md | xargs sed -i 's/\bmake_str\b/ensure_str/g' ) ;;
  t5*) ANSWER='src/click/core.py, class Command, method main' ;;
  t6*) ANSWER='Root cause: db connection pool exhausted. INC-7741' ;;
  t7*) ANSWER=$(cd $W/click; git log --format=%H --invert-grep --grep='^wip' -1 -- src/click/formatting.py) ;;
  t8*) ( cd $W/express; sed -i 's/app.listen(3000)/app.listen(3001)/' examples/hello-world/index.js ) ;;
  s1*) ANSWER=$(cd $W/click; python -m pytest -q -p no:cacheprovider tests 2>&1 | grep '^FAILED' | sed 's/^FAILED //; s/::.*//' | sort -u | tr '
' ' ') ;;
  s2*) ANSWER='test fails: expected 418 "I am a Teapot", got 200 OK' ;;
  s3*) ANSWER='"Value must be an iterable." and "Takes {nargs} values but {len} were given."' ;;
  s4*) ANSWER='Root cause: db connection pool exhausted. INC-7741' ;;
esac
echo "$t solved:      $(ANSWER=$ANSWER grade_$t 2>&1 | tail -1 | cut -c1-100)"
