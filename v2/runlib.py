"""Host-side helpers for run_v2.sh (pure stdlib). Subcommands:
  schedule                 -> full v2 schedule, one 'block arm task rep' per line (fixed seeds, block order)
  classify DIR             -> one line: OK | RATELIMIT <reset_epoch|0> <reason> | TRANSIENT <reason> | ABORT <reason>
  verify DIR ARM           -> writes DIR/verify.txt (arm activity + auth checks), prints it on one line
  line DIR                 -> short result line for the progress log
"""
import datetime as dt, glob, json, os, random, re, sys

ARMS = ["baseline", "ojas3", "headroom", "rtk"]
BLOCKS = [("read", ["r1_pricing_policy", "r2_ledger_raises", "r3_click_core_bug"], 10, 2026112101),
          ("stress", ["s1_click_full_suite", "s2_express_full_suite", "s3_click_core_full_read", "s4_log_cat_all"], 10, 2026112102),
          ("main", ["t1_click_bug", "t2_express_bug", "t3_walkdir_bug", "t4_click_rename", "t5_click_qa",
                    "t6_log_rootcause", "t7_git_history", "t8_control_edit"], 10, 2026112103),
          ("long", ["l1_click_duration", "l2_express_ci", "l3_click_split_core"], 10, 2026112104)]
# Subscription auth reports apiKeySource "none" in the init event; anything else means an API key path.
ALLOWED_KEY_SOURCES = {"none"}
LIMIT_RE = re.compile(r"usage limit|rate limit|rate_limit|limit reached|hit your limit|limit will reset|resets? (at )?\d|out of extra usage|\b429\b", re.I)
TRANSIENT_RE = re.compile(r"overloaded|\b529\b|\b5\d\d\b.*error|ECONNRESET|ETIMEDOUT|socket hang up|network error|fetch failed|Connection error", re.I)


def schedule():
    for block, tasks, reps, seed in BLOCKS:
        jobs = [(a, t, r) for r in range(1, reps + 1) for t in tasks for a in ARMS]
        random.Random(seed).shuffle(jobs)
        for a, t, r in jobs:
            print(block, a, t, r)


def _read(p):
    try:
        return open(p, encoding="utf-8", errors="replace").read()
    except OSError:
        return ""


def _json(p):
    try:
        return json.loads(_read(p) or "null")
    except ValueError:
        return None


def parse_reset(text, now=None):
    """Return a reset time (epoch) from a limit message, or 0 if none can be parsed."""
    now = now or dt.datetime.now(dt.timezone.utc)
    m = re.search(r"\|(\d{10})\b", text) or re.search(r"reset[s]?[^0-9]{0,20}(\d{10})\b", text)
    if m:
        return int(m.group(1))
    m = re.search(r"resets?\s+(?:at\s+)?(?:(\w{3,9}\s+\d{1,2}),?\s+(?:at\s+)?)?(\d{1,2})(?::(\d{2}))?\s*(am|pm)?\s*(?:\(([A-Za-z_/+\-0-9]+)\))?", text, re.I)
    if not m:
        return 0
    datestr, hh, mm, ampm, tzname = m.groups()
    h, mi = int(hh), int(mm or 0)
    if ampm:
        h = h % 12 + (12 if ampm.lower() == "pm" else 0)
    if h > 23 or mi > 59:
        return 0
    tz = dt.timezone.utc  # the CLI runs in the container, whose local time is UTC
    if tzname and tzname.upper() not in ("UTC", "GMT"):
        try:
            from zoneinfo import ZoneInfo
            tz = ZoneInfo(tzname)
        except Exception:
            pass
    local_now = now.astimezone(tz)
    cand = local_now.replace(hour=h, minute=mi, second=0, microsecond=0)
    if datestr:
        try:
            d = dt.datetime.strptime(f"{datestr} {local_now.year}", "%b %d %Y")
            cand = cand.replace(month=d.month, day=d.day)
            if cand < local_now - dt.timedelta(days=1):
                cand = cand.replace(year=cand.year + 1)
        except ValueError:
            pass
    while cand <= local_now:
        cand += dt.timedelta(days=1)
    return int(cand.timestamp())


def classify(d):
    if os.path.exists(f"{d}/ABORT"):
        return "ABORT " + _read(f"{d}/ABORT").strip().replace("\n", " ")
    init = _json(f"{d}/init.json") or {}
    if init and init.get("apiKeySource") not in ALLOWED_KEY_SOURCES:
        return f"ABORT non-subscription auth: apiKeySource={init.get('apiKeySource')!r}"
    j = _json(f"{d}/run.json")
    stderr = _read(f"{d}/stderr.txt")
    if isinstance(j, dict):
        res = str(j.get("result", ""))
        status = j.get("api_error_status")
        if j.get("is_error") and (status == 429 or LIMIT_RE.search(res)):
            return f"RATELIMIT {parse_reset(res + ' ' + stderr)} {res[:120]!r}".replace("\n", " ")
        if j.get("is_error") and status in (401, 403):
            return f"ABORT auth error {status}: {res[:160]!r}".replace("\n", " ")
        if j.get("is_error") and j.get("subtype") != "timeout" and (TRANSIENT_RE.search(res) or (isinstance(status, int) and status >= 500)):
            return f"TRANSIENT api error {status}: {res[:120]!r}".replace("\n", " ")
        if not os.path.exists(f"{d}/grade.txt") or not _read(f"{d}/grade.txt").strip():
            return "TRANSIENT no grade"
        return "OK"
    # no result object at all: limit text on stderr / in the stream, or an infrastructure failure
    raw = _read(f"{d}/run.raw")[-20000:]
    blob = stderr + "\n" + raw
    if LIMIT_RE.search(stderr) or re.search(r"usage limit|limit reached|hit your limit", raw, re.I):
        return f"RATELIMIT {parse_reset(blob)} {(stderr.strip() or raw.strip())[-160:]!r}".replace("\n", " ")
    if re.search(r"invalid api key|authentication|unauthori[sz]ed|\b401\b|OAuth token", blob, re.I):
        return "ABORT auth failure: " + blob.strip()[-200:].replace("\n", " ")
    return "TRANSIENT no result: " + (stderr.strip()[-160:] or "empty stderr").replace("\n", " ")


def verify(d, arm):
    init = _json(f"{d}/init.json") or {}
    settings = _read(f"{d}/settings.json")
    tr = ""
    for f in glob.glob(f"{d}/transcript/**/*.jsonl", recursive=True):
        tr += _read(f)
    ts = _read(f"{d}/tool_stats.txt")
    v = {"apiKeySource": init.get("apiKeySource"), "model": init.get("model"), "cc_version": init.get("claude_code_version")}
    ok = init.get("apiKeySource") in ALLOWED_KEY_SOURCES
    note = "ojas get" in tr  # ojas SessionStart note
    style = "Response style" in tr
    rtk_hook = '"rtk hook claude"' in settings
    ojas_hook = "post-tool-use" in settings and "Bash|Grep|Read" in settings
    v.update(ojas_hooks=int(ojas_hook), ojas_session_note=int(note), response_style_text=int(style), rtk_hook=int(rtk_hook))
    if arm == "ojas3":
        st = [json.loads(x) for x in _read(f"{d}/ojas_stats.jsonl").splitlines() if x.strip().startswith("{")]
        v["ojas_stats_entries"] = len(st)
        v["ojas_read_tool_outlines"] = sum(1 for s in st if "Read tool" in json.dumps(s))
        v["ojas_grep_tool"] = sum(1 for s in st if "Grep tool" in json.dumps(s))
        ok = ok and ojas_hook and note and not style
    elif arm == "rtk":
        m = re.search(r"Total commands:\s*([\d,]+)", ts)
        v["rtk_commands"] = int(m.group(1).replace(",", "")) if m else 0
        m = re.search(r"Tokens saved:\s*([\d.]+[KM]?)", ts)
        v["rtk_tokens_saved"] = m.group(1) if m else "0"
        ok = ok and rtk_hook and not ojas_hook
    elif arm == "headroom":
        v["headroom_perf_nonempty"] = int(bool(re.search(r"request|saved|token", ts, re.I)))
        ok = ok and not ojas_hook and not rtk_hook and bool(re.search(r"headroom", _read(f"{d}/run.raw")[:5000] + ts, re.I))
    elif arm == "baseline":
        ok = ok and not ojas_hook and not rtk_hook and not note
    v["valid"] = int(bool(ok))
    line = " ".join(f"{k}={v[k]}" for k in v)
    open(f"{d}/verify.txt", "w", encoding="utf-8").write(line + "\n")
    return line


def line(d):
    j = _json(f"{d}/run.json") or {}
    g = (_read(f"{d}/grade.txt").strip().splitlines() or ["?"])[-1][:60]
    cap = " MAX_TURNS" if j.get("subtype") == "error_max_turns" else ""
    return f"{g} cost={j.get('total_cost_usd', 0) or 0:.4f} turns={j.get('num_turns')}{cap}"


if __name__ == "__main__":
    cmd = sys.argv[1]
    if cmd == "schedule":
        schedule()
    elif cmd == "classify":
        print(classify(sys.argv[2]))
    elif cmd == "verify":
        print(verify(sys.argv[2], sys.argv[3]))
    elif cmd == "line":
        print(line(sys.argv[2]))
    elif cmd == "reset-test":
        print(parse_reset(sys.argv[2]))
