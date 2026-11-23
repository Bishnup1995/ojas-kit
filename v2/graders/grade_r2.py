"""r2 grader: the agent's final answer must name exactly the expected set of functions (order, bullets, backticks
and trailing '()' ignored; any other identifier-only line counts as an extra). Usage: ANSWER=... grade_r2.py EXPECTED_FILE"""
import os, re, sys
exp = set(open(sys.argv[1]).read().split())
names = set()
for line in os.environ.get("ANSWER", "").splitlines():
    m = re.fullmatch(r"[\s\-\*\d\.\)`]*([A-Za-z_][A-Za-z0-9_]*)(\(\))?`?\s*", line)
    if m: names.add(m.group(1))
miss, extra = sorted(exp - names), sorted(names - exp)
print("PASS" if not miss and not extra else f"FAIL missing={miss} extra={extra}"[:300])
