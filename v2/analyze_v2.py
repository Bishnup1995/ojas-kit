"""Benchmark v2 analysis. Safe to run at any time on partial results.
  python analyze_v2.py [RAW_DIR]     (default ../results/v2/raw; writes ../results/v2/summary.md when run on raw)
Per block and arm: n, passes, mean cost (Claude Code's list-price estimate total_cost_usd, not a bill), turns,
input / cache-write / cache-read / output tokens, cost per correct answer, max-turn caps, timeouts.
Ratios vs baseline: ratio of task-balanced means (mean over tasks of the per-task mean), with a 95% interval from a
bootstrap stratified by task (runs resampled within each task and arm; tasks are the pairing unit), 5000 resamples,
seed 1. Pass rates: two-sided Fisher exact test vs baseline. Runs whose verify.txt says valid=0 are excluded and listed.
Timeouts (subtype "timeout") count as failures; they have no token data and are excluded from token/cost means."""
import collections, glob, json, math, os, random, re, statistics as st, sys

RAW = sys.argv[1] if len(sys.argv) > 1 else os.path.join(os.path.dirname(os.path.abspath(__file__)), "../results/v2/raw")
ARMS = ["baseline", "ojas3", "headroom", "rtk"]
BLOCKS = ["read", "stress", "main", "long", "smoke"]
def block_of(t): return {"r": "read", "s": "stress", "t": "main", "l": "long"}.get(t[0], "?")

def merge_results(rs):
    """Claude Code emits a second `result` event when a background task the agent started ends after the task.
    num_turns, usage and duration_ms are per segment; total_cost_usd is cumulative. Sum the former, keep the last cost."""
    j = dict(rs[-1])
    if len(rs) > 1:
        j["num_turns"] = sum(r.get("num_turns") or 0 for r in rs)
        j["duration_ms"] = sum(r.get("duration_ms") or 0 for r in rs)
        j["result"] = "\n\n".join(r.get("result") or "" for r in rs)
        u = {}
        for r in rs:
            for k, x in (r.get("usage") or {}).items():
                if isinstance(x, (int, float)): u[k] = u.get(k, 0) + x
        j["usage"] = u
        j["n_results"] = len(rs)
    return j

def load_result(d):
    rs = []
    if os.path.exists(f"{d}/run.raw"):
        for l in open(f"{d}/run.raw", encoding="utf-8", errors="replace"):
            if l.startswith("{") and '"type":"result"' in l:
                try: rs.append(json.loads(l))
                except ValueError: pass
    j = json.load(open(f"{d}/run.json", encoding="utf-8"))
    if len(rs) > 1 and j.get("subtype") != "timeout":
        return merge_results(rs)
    return j

runs, invalid, incomplete = [], [], []
for d in sorted(glob.glob(os.path.join(RAW, "*__*__*"))):
    arm, task, rep = os.path.basename(d).split("__")
    try:
        j = load_result(d)
        g = open(f"{d}/grade.txt", encoding="utf-8", errors="replace").read().strip().splitlines()[-1]
        v = open(f"{d}/verify.txt", encoding="utf-8").read()
    except Exception:
        incomplete.append(os.path.basename(d)); continue
    if "valid=1" not in v:
        invalid.append(f"{os.path.basename(d)}: {v.strip()}"); continue
    u = j.get("usage") or {}
    to = j.get("subtype") == "timeout"
    vv = dict(re.findall(r"(\w+)=(\S+)", v))
    runs.append(dict(arm=arm, task=task, rep=rep, block=block_of(task) if "smoke" not in RAW else "smoke",
        passed=g.startswith("PASS"), timeout=to, cap=j.get("subtype") == "error_max_turns",
        cost=None if to else (j.get("total_cost_usd") or 0), turns=None if to else j.get("num_turns", 0),
        inp=u.get("input_tokens", 0), cw=u.get("cache_creation_input_tokens", 0), cr=u.get("cache_read_input_tokens", 0),
        out=u.get("output_tokens", 0), wall=(j.get("duration_ms") or 0) / 1000, v=vv))

def mean(xs):
    xs = [x for x in xs if x is not None]
    return st.mean(xs) if xs else float("nan")

def fisher(a, n1, b, n2):
    """Two-sided Fisher exact p for a/n1 passes vs b/n2 passes."""
    k, N = a + b, n1 + n2
    if N == 0: return float("nan")
    def p(x): return math.comb(n1, x) * math.comb(n2, k - x) / math.comb(N, k)
    p0 = p(a)
    return min(1.0, sum(p(x) for x in range(max(0, k - n2), min(k, n1) + 1) if p(x) <= p0 * (1 + 1e-9)))

def balanced(rs_by_task, f):
    ms = [mean(r[f] for r in rs) for rs in rs_by_task.values() if rs]
    ms = [m for m in ms if not math.isnan(m)]
    return st.mean(ms) if ms else float("nan")

def ratio_ci(R, arm, f, rng):
    tasks = sorted({r["task"] for r in R if r["arm"] == arm} & {r["task"] for r in R if r["arm"] == "baseline"})
    if not tasks: return None
    A = {t: [r for r in R if r["arm"] == arm and r["task"] == t and r[f] is not None] for t in tasks}
    B = {t: [r for r in R if r["arm"] == "baseline" and r["task"] == t and r[f] is not None] for t in tasks}
    if any(not A[t] or not B[t] for t in tasks): return None
    pt = balanced(A, f) / max(1e-12, balanced(B, f))
    bs = []
    for _ in range(5000):
        a = {t: rng.choices(A[t], k=len(A[t])) for t in tasks}; b = {t: rng.choices(B[t], k=len(B[t])) for t in tasks}
        bs.append(balanced(a, f) / max(1e-12, balanced(b, f)))
    bs.sort()
    return pt, bs[124], bs[4874], len(tasks)

rng = random.Random(1)
L = [f"# Benchmark v2 summary ({len(runs)} valid runs, {len(invalid)} invalid, {len(incomplete)} incomplete) from {os.path.relpath(RAW)}", ""]
for blk in BLOCKS:
    R = [r for r in runs if r["block"] == blk]
    if not R: continue
    tasks = sorted({r["task"] for r in R})
    L += [f"## Block: {blk}  ({len(R)} runs, tasks: {', '.join(tasks)})", "",
          "| arm | n | pass | mean cost $ | cost/correct $ | turns | input | cache write | cache read | output | wall s | caps | timeouts |",
          "|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|"]
    for a in ARMS:
        rs = [r for r in R if r["arm"] == a]
        if not rs: continue
        np_ = sum(r["passed"] for r in rs); costs = [r["cost"] for r in rs if r["cost"] is not None]
        L.append(f"| {a} | {len(rs)} | {np_}/{len(rs)} | {mean(costs):.4f} | {(sum(costs)/np_ if np_ else float('nan')):.4f} | "
                 f"{mean(r['turns'] for r in rs):.1f} | {mean(r['inp'] for r in rs if not r['timeout']):.0f} | {mean(r['cw'] for r in rs if not r['timeout']):.0f} | "
                 f"{mean(r['cr'] for r in rs if not r['timeout']):.0f} | {mean(r['out'] for r in rs if not r['timeout']):.0f} | {mean(r['wall'] for r in rs):.0f} | "
                 f"{sum(r['cap'] for r in rs)} | {sum(r['timeout'] for r in rs)} |")
    if any(r["arm"] == "baseline" for r in R):
        L += ["", "Ratio vs baseline, task-balanced means, 95% stratified bootstrap interval; pass rate Fisher exact p:", "",
              "| arm | cost | turns | cache read | cache write | output | pass rate (p) |", "|---|---|---|---|---|---|---|"]
        b = [r for r in R if r["arm"] == "baseline"]
        for a in ARMS[1:]:
            rs = [r for r in R if r["arm"] == a]
            if not rs: continue
            cells = []
            for f in ("cost", "turns", "cr", "cw", "out"):
                x = ratio_ci(R, a, f, rng)
                cells.append(f"{x[0]:.2f}x [{x[1]:.2f}, {x[2]:.2f}]" if x else "-")
            pa, pb = sum(r["passed"] for r in rs), sum(r["passed"] for r in b)
            cells.append(f"{pa}/{len(rs)} vs {pb}/{len(b)} (p={fisher(pa, len(rs), pb, len(b)):.3f})")
            L.append(f"| {a} | " + " | ".join(cells) + " |")
    L += ["", "Per task: mean cost $ / mean turns (passes/n)", "", "| task | " + " | ".join(ARMS) + " |", "|---|" + "---|" * len(ARMS)]
    for t in tasks:
        cells = []
        for a in ARMS:
            rs = [r for r in R if r["arm"] == a and r["task"] == t]
            cells.append(f"{mean(r['cost'] for r in rs):.4f} / {mean(r['turns'] for r in rs):.1f} ({sum(r['passed'] for r in rs)}/{len(rs)})" if rs else "-")
        L.append(f"| {t} | " + " | ".join(cells) + " |")
    act = []
    for a, keys in (("ojas3", ("ojas_stats_entries", "ojas_read_tool_outlines", "ojas_grep_tool")), ("rtk", ("rtk_commands",))):
        rs = [r for r in R if r["arm"] == a]
        if rs:
            act.append(f"{a}: " + ", ".join(f"{k} total={sum(int(r['v'].get(k, 0)) for r in rs)} (runs>0: {sum(int(r['v'].get(k, 0)) > 0 for r in rs)}/{len(rs)})" for k in keys))
    if act: L += ["", "Tool activity: " + "; ".join(act)]
    L.append("")
L += ["Invalid runs (excluded): " + ("; ".join(invalid) if invalid else "none"),
      "Incomplete dirs (in progress or failed infra): " + (", ".join(incomplete) if incomplete else "none")]
txt = "\n".join(L) + "\n"
print(txt)
if os.path.basename(os.path.normpath(RAW)) == "raw":
    open(os.path.join(os.path.dirname(os.path.normpath(RAW)), "summary.md"), "w", encoding="utf-8").write(txt)
