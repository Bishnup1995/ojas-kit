"""Layer 1 scorer. Arm-blind: same tokenizer, same fact extraction for every arm.
Usage: python score.py  -> ../results/layer1.csv and ../results/layer1.md"""
import csv, glob, os, re, statistics
import tiktoken
enc = tiktoken.get_encoding("o200k_base")
tok = lambda s: len(enc.encode(s, disallowed_special=()))
ARMS = ["baseline", "ojas", "caveman", "headroom"]
# Fact lines = primary failure/summary identifiers only (see PROTOCOL CHANGELOG: scorer fix 2026-11-20).
FACT = re.compile(r"^(FAILED |ERROR |=+ .*\b(failed|passed|error)\b.* =+$|\s*\d+ (failing|passing)|\s*\d+\) \S|test .* \.\.\. FAILED|.*panicked at|test result: |.*\bFATAL\b)")
HANDLE = re.compile(r"ojas get|ccr_[0-9a-f]+|caveman_retrieve|headroom_retrieve|hash=|retrieve", re.I)
norm = lambda s: re.sub(r"\s+", " ", s).strip()
rd = lambda p: open(p, encoding="utf-8", errors="replace").read() if os.path.exists(p) else ""
rows = []
for f in sorted(glob.glob("corpus/*.txt")):
    n = os.path.basename(f)[:-4]; raw = rd(f)
    facts = sorted({norm(l) for l in raw.splitlines() if FACT.search(l) and len(norm(l)) > 8})[:200]
    base_t = tok(raw)
    for a in ARMS:
        out = rd(f"out/{a}/{n}.txt"); t = tok(out)
        o = norm(out)
        kept = sum(1 for x in facts if x in o)
        rec = bool(HANDLE.search(out))
        rows.append(dict(item=n, arm=a, raw_tokens=base_t, out_tokens=t,
            saved_pct=round(100 * (1 - t / base_t), 1) if base_t else 0.0,
            facts=len(facts), facts_verbatim=kept, recoverable=int(rec)))
os.makedirs("../results", exist_ok=True)
with open("../results/layer1.csv", "w", newline="") as fh:
    w = csv.DictWriter(fh, fieldnames=list(rows[0])); w.writeheader(); w.writerows(rows)
items = sorted({r["item"] for r in rows})
by = {(r["item"], r["arm"]): r for r in rows}
L = ["| item | raw tok | " + " | ".join(ARMS[1:]) + " |", "|---|---:|" + "---:|" * 3]
for i in items:
    L.append(f"| {i} | {by[(i,'baseline')]['raw_tokens']} | " + " | ".join(f"{by[(i,a)]['out_tokens']} ({by[(i,a)]['saved_pct']:.0f}%)" for a in ARMS[1:]) + " |")
L += ["", "| arm | total tokens | saved (total) | median saved/item | items >15% saved | fact lines kept verbatim | items w/ recovery handle |", "|---|---:|---:|---:|---:|---:|---:|"]
raw_total = sum(by[(i, "baseline")]["raw_tokens"] for i in items)
for a in ARMS:
    rs = [by[(i, a)] for i in items]
    tot = sum(r["out_tokens"] for r in rs); ft = sum(r["facts"] for r in rs); fk = sum(r["facts_verbatim"] for r in rs)
    L.append(f"| {a} | {tot} | {100*(1-tot/raw_total):.1f}% | {statistics.median(r['saved_pct'] for r in rs):.1f}% | {sum(1 for r in rs if r['saved_pct']>15)} | {fk}/{ft} | {sum(r['recoverable'] for r in rs)} |")
open("../results/layer1.md", "w").write("\n".join(L) + "\n")
print("\n".join(L[-6:]))
