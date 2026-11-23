"""Analysis for the layer2_styleoff follow-up. Same logic as analyze.py (cost = Claude Code's list-price estimate
total_cost_usd; paired bootstrap of per-task mean-cost ratios vs baseline, seed 1, 5000 resamples), reading
../results/layer2_styleoff/raw and writing ../results/layer2_styleoff/summary.md. Also reports ojas compression
counts (tool_stats.txt) and the per-run arm verification (verify.txt)."""
import glob, json, os, random, re, statistics as st, collections
BASE = "../results/layer2_styleoff"
R = collections.defaultdict(list); comp = collections.Counter(); bad = []
for d in sorted(glob.glob(f"{BASE}/raw/*__*__*")):
    arm, task, rep = os.path.basename(d).split("__")
    try: j = json.load(open(f"{d}/run.json", encoding="utf-8")); g = open(f"{d}/grade.txt", encoding="utf-8").read().strip().splitlines()[-1]
    except Exception: bad.append(os.path.basename(d) + " (no run.json/grade)"); continue
    u = j.get("usage", {})
    R[(arm, task)].append(dict(rep=rep, passed=g.startswith("PASS"), cost=j.get("total_cost_usd", 0) or 0,
        turns=j.get("num_turns", 0), ms=j.get("duration_ms", 0), inp=u.get("input_tokens", 0),
        cw=u.get("cache_creation_input_tokens", 0), cr=u.get("cache_read_input_tokens", 0), out=u.get("output_tokens", 0)))
    if arm.startswith("ojas"):
        ts = open(f"{d}/tool_stats.txt", encoding="utf-8", errors="replace").read()
        m = re.search(r"runs (\d+)\s+compressed (\d+)", ts)
        comp[(arm, "runs")] += int(m.group(1)) if m else 0; comp[(arm, "compressed")] += int(m.group(2)) if m else 0
    v = open(f"{d}/verify.txt").read() if os.path.exists(f"{d}/verify.txt") else ""
    sf = re.search(r"style_full_in_transcript=(\d+)", v); hk = re.search(r"hook_note_in_transcript=(\d+)", v)
    sf, hk = (int(sf.group(1)) if sf else -1), (int(hk.group(1)) if hk else -1)
    want = {"baseline": (0, 0), "ojas": (1, 1), "ojas-off": (0, 1)}[arm]
    if (min(sf, 1), min(hk, 1)) != want: bad.append(f"{os.path.basename(d)} verify mismatch: {v.strip()}")
arms = ["baseline", "ojas", "ojas-off"]; tasks = sorted({t for _, t in R})
def agg(arm, f, ts=tasks): return [r[f] for t in ts for r in R.get((arm, t), [])]
rows = ["| arm | runs | pass | mean cost $ | mean turns | mean input | mean cache-write | mean cache-read | mean output | mean wall s |",
        "|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|"]
for a in arms:
    p = agg(a, "passed")
    if not p: continue
    rows.append(f"| {a} | {len(p)} | {sum(p)}/{len(p)} | {st.mean(agg(a,'cost')):.4f} | {st.mean(agg(a,'turns')):.2f} | {st.mean(agg(a,'inp')):.0f} | "
                f"{st.mean(agg(a,'cw')):.0f} | {st.mean(agg(a,'cr')):.0f} | {st.mean(agg(a,'out')):.0f} | {st.mean(agg(a,'ms'))/1000:.0f} |")
rows += ["", "Per task, mean cost $ / mean turns (pass):", "", "| task | " + " | ".join(arms) + " |", "|---|" + "---:|" * len(arms)]
for t in tasks:
    cells = []
    for a in arms:
        rs = R.get((a, t), [])
        cells.append(f"{st.mean(r['cost'] for r in rs):.4f} / {st.mean(r['turns'] for r in rs):.1f} ({sum(r['passed'] for r in rs)}/{len(rs)})" if rs else "-")
    rows.append(f"| {t} | " + " | ".join(cells) + " |")
rng = random.Random(1)
rows += ["", "Cost ratio vs baseline (paired bootstrap over task means, 95% interval):", ""]
def boot(a, b):
    ts = [t for t in tasks if R.get((a, t)) and R.get((b, t))]
    ratios = [st.mean(r["cost"] for r in R[(a, t)]) / max(1e-9, st.mean(r["cost"] for r in R[(b, t)])) for t in ts]
    bs = sorted(st.mean(rng.choices(ratios, k=len(ratios))) for _ in range(5000))
    return f"{st.mean(ratios):.2f}x  [{bs[125]:.2f}, {bs[4875]:.2f}]  over {len(ts)} tasks"
for a in arms[1:]: rows.append(f"- {a} / baseline: {boot(a, 'baseline')}")
rows.append(f"- ojas / ojas-off: {boot('ojas', 'ojas-off')}")
rows += ["", "ojas compression (tool_stats.txt, summed over runs): " +
         ", ".join(f"{a}: {comp[(a,'compressed')]} compressed of {comp[(a,'runs')]} commands through ojas" for a in arms[1:])]
rows += ["", f"Total estimated cost: ${sum(agg(a,'cost') and sum(agg(a,'cost')) or 0 for a in arms):.4f}",
         "", "Verification problems: " + ("; ".join(bad) if bad else "none (baseline: no ojas text; ojas: style+note; ojas-off: note only)")]
open(f"{BASE}/summary.md", "w", encoding="utf-8").write("\n".join(rows) + "\n"); print("\n".join(rows))
