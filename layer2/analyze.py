"""Layer 2 analysis. Reads results/layer2/raw/*/{run.json,grade.txt}. Cost is Claude Code's own list-price estimate
(total_cost_usd), not a bill. Paired bootstrap vs baseline over tasks."""
import glob, json, os, random, statistics as st, collections
R = collections.defaultdict(list)
for d in sorted(glob.glob("../results/layer2/raw/*__*__*")):
    arm, task, rep = os.path.basename(d).split("__")
    try: j = json.load(open(f"{d}/run.json", encoding="utf-8")); g = open(f"{d}/grade.txt", encoding="utf-8").read().strip().splitlines()[-1]
    except Exception: continue
    u = j.get("usage", {})
    R[(arm, task)].append(dict(rep=rep, passed=g.startswith("PASS"), cost=j.get("total_cost_usd", 0) or 0,
        turns=j.get("num_turns", 0), ms=j.get("duration_ms", 0), inp=u.get("input_tokens", 0),
        cw=u.get("cache_creation_input_tokens", 0), cr=u.get("cache_read_input_tokens", 0), out=u.get("output_tokens", 0)))
arms = ["baseline", "ojas", "caveman", "headroom"]; tasks = sorted({t for _, t in R})
def agg(arm, f, ts=tasks): return [r[f] for t in ts for r in R.get((arm, t), [])]
rows = ["| arm | runs | pass rate | mean cost $ | cost per pass $ | mean turns | mean cache-read tok | mean output tok | mean wall s |", "|---|---:|---:|---:|---:|---:|---:|---:|---:|"]
for a in arms:
    p = agg(a, "passed")
    if not p: continue
    c = agg(a, "cost"); npass = sum(p)
    rows.append(f"| {a} | {len(p)} | {100*npass/len(p):.0f}% | {st.mean(c):.3f} | {(sum(c)/npass if npass else float('nan')):.3f} | {st.mean(agg(a,'turns')):.1f} | {st.mean(agg(a,'cr')):.0f} | {st.mean(agg(a,'out')):.0f} | {st.mean(agg(a,'ms'))/1000:.0f} |")
rows += ["", "Per task, mean cost $ (pass rate):", "", "| task | " + " | ".join(arms) + " |", "|---|" + "---:|" * len(arms)]
for t in tasks:
    cells = []
    for a in arms:
        rs = R.get((a, t), [])
        cells.append(f"{st.mean(r['cost'] for r in rs):.3f} ({sum(r['passed'] for r in rs)}/{len(rs)})" if rs else "-")
    rows.append(f"| {t} | " + " | ".join(cells) + " |")
rng = random.Random(1)
rows += ["", "Cost ratio vs baseline (paired bootstrap over task means, 95% interval):", ""]
for a in arms[1:]:
    ts = [t for t in tasks if R.get((a, t)) and R.get(("baseline", t))]
    if not ts: continue
    ratios = [st.mean(r["cost"] for r in R[(a, t)]) / max(1e-9, st.mean(r["cost"] for r in R[("baseline", t)])) for t in ts]
    bs = sorted(st.mean(rng.choices(ratios, k=len(ratios))) for _ in range(5000))
    rows.append(f"- {a}: {st.mean(ratios):.2f}x  [{bs[125]:.2f}, {bs[4875]:.2f}]  over {len(ts)} tasks")
os.makedirs("../results", exist_ok=True)
open("../results/layer2.md", "w").write("\n".join(rows) + "\n"); print("\n".join(rows))
