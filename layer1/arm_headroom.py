# Runs inside the headroom image with its own venv python. Each item is a tool_result in an Anthropic-format message list.
import glob, json, os
from headroom import compress
for f in sorted(glob.glob('/in/*.txt')):
    n = os.path.basename(f)[:-4]
    raw = open(f, encoding='utf-8', errors='replace').read()
    msgs = [
        {"role": "user", "content": "Run the command and tell me what you find."},
        {"role": "assistant", "content": [{"type": "tool_use", "id": "t1", "name": "Bash", "input": {"command": open(f[:-4] + '.cmd').read().strip()}}]},
        {"role": "user", "content": [{"type": "tool_result", "tool_use_id": "t1", "content": raw}]},
    ]
    try:
        r = compress(msgs, model="claude-sonnet-4-5")
        c = r.messages[-1]["content"]
        out = c[0]["content"] if isinstance(c, list) else c
        if isinstance(out, list): out = "".join(p.get("text", "") for p in out)
        open(f'/out/{n}.txt', 'w').write(out)
        json.dump({"tokens_saved": r.tokens_saved}, open(f'/out/{n}.meta', 'w'))
    except Exception as e:
        open(f'/out/{n}.txt', 'w').write(raw); open(f'/out/{n}.meta', 'w').write(json.dumps({"error": repr(e)}))
