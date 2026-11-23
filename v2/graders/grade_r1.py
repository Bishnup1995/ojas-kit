"""r1 grader: the agent's pricing.py must agree with the correct reference on every public function, over a
fixed grid of inputs (boundaries included). Usage: grade_r1.py AGENT_PRICING_PY REFERENCE_PRICING_PY"""
import importlib.util, sys
from decimal import Decimal
def load(p, name):
    spec = importlib.util.spec_from_file_location(name, p); m = importlib.util.module_from_spec(spec); spec.loader.exec_module(m); return m
try:
    a = load(sys.argv[1], "agent_pricing")
except Exception as e:
    print(f"FAIL import: {e!r}"[:200]); sys.exit(0)
r = load(sys.argv[2], "ref_pricing")
qtys = [1, 24, 25, 99, 100, 101, 150, 499, 500, 501, 999, 1000, 1001, 2500]
bad = []
for name in sorted(n for n in dir(r) if n.startswith("rate_")):
    for q in qtys:
        try: got = getattr(a, name)(q, Decimal("3.17"))
        except Exception as e: got = repr(e)
        if got != getattr(r, name)(q, Decimal("3.17")): bad.append(f"{name}({q})")
skus = ["bolts-north", "valves-alpine", "cables-coastal", "springs-east", "hinges-west"]
for sku in skus:
    for q in qtys:
        for y in (0, 3, 10):
            for region in ("north", "alpine", "coastal"):
                try: got = a.quote([(sku, q), ("nuts-south", 7)], region, y)
                except Exception as e: got = repr(e)
                if got != r.quote([(sku, q), ("nuts-south", 7)], region, y): bad.append(f"quote({sku},{q},{y},{region})")
print("PASS" if not bad else f"FAIL {len(bad)} mismatches, e.g. {bad[:3]}")
