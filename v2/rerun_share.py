"""Share of tool-output tokens that comes from repeated runs of a test command (FOLLOWUP3_PREREG.md).

Usage: python rerun_share.py <raw dir> [arm]
A Bash call counts as a test run when it invokes pytest, npm test, mocha or cargo test. It is a repeat when an
earlier call in the same run invoked the same test runner. Tokens are estimated as characters / 4.
"""
import glob, json, os, re, sys, statistics as st

RAW = sys.argv[1]
ARM = sys.argv[2] if len(sys.argv) > 2 else "baseline"
TEST = re.compile(r"\b(pytest|npm (?:run )?test|mocha|cargo test)\b")

def text_of(c):
    if isinstance(c, str):
        return c
    if isinstance(c, list):
        return "".join(x.get("text", "") for x in c if isinstance(x, dict))
    return ""

rows = []
for d in sorted(glob.glob(os.path.join(RAW, f"{ARM}__*__*"))):
    cmds, total, rep, tests = {}, 0, 0, 0
    seen_runners = set()
    files = [f for f in glob.glob(os.path.join(d, "transcript", "**", "*.jsonl"), recursive=True) if "subagents" not in f]
    files = sorted(files, key=os.path.getsize)[-1:]  # the main session
    for f in files:
        for line in open(f, encoding="utf-8", errors="replace"):
            try:
                e = json.loads(line)
            except ValueError:
                continue
            for c in (e.get("message") or {}).get("content") or []:
                if not isinstance(c, dict):
                    continue
                if c.get("type") == "tool_use" and c.get("name") == "Bash":
                    cmds[c["id"]] = c.get("input", {}).get("command", "")
                elif c.get("type") == "tool_result":
                    n = len(text_of(c.get("content"))) / 4
                    total += n
                    cmd = cmds.get(c.get("tool_use_id"), "")
                    m = TEST.search(cmd)
                    if m:
                        if m.group(1) in seen_runners:
                            rep += n
                        seen_runners.add(m.group(1))
                        tests += 1
    rows.append((os.path.basename(d), total, rep, tests))

if not rows:
    sys.exit("no runs")
for name, total, rep, tests in rows:
    print(f"{name:45s} tool-output ~{total:8.0f} tok  repeated test runs ~{rep:7.0f} tok ({100 * rep / max(total, 1):4.1f}%)  test calls {tests}")
shares = [r[2] / max(r[1], 1) for r in rows]
print(f"median share {100 * st.median(shares):.1f}%, pooled {100 * sum(r[2] for r in rows) / max(sum(r[1] for r in rows), 1):.1f}%")
