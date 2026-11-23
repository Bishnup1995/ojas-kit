"""l2 setup: run the (broken) express test suite and wrap it in a realistic CI job log at ~/work/ci/ci.log."""
import os, random, subprocess
rng = random.Random(4242)
os.makedirs("/home/bench/work/ci", exist_ok=True)
t = [0.0]
def ts():
    t[0] += rng.random() * 0.4
    return f"2026-11-21T08:{14 + int(t[0]) // 60 % 40:02d}:{int(t[0]) % 60:02d}.{rng.randint(0, 9999999):07d}Z "
L = ["##[group]Run actions/checkout@v4", "Syncing repository: expressjs/express", "Getting Git version info",
     "Temporarily overriding HOME='/home/runner/work/_temp' before making global git config changes", "##[endgroup]",
     "##[group]Run actions/setup-node@v4", "Found in cache @ /opt/hostedtoolcache/node/22.11.0/x64", "##[endgroup]",
     "##[group]Run npm install --no-audit --no-fund"]
pk = ["accepts", "body-parser", "debug", "ms", "qs", "send", "mime-types", "mime-db", "statuses", "http-errors", "router",
      "path-to-regexp", "finalhandler", "on-finished", "ee-first", "type-is", "media-typer", "content-type", "etag", "fresh"]
for i in range(900):
    p = rng.choice(pk)
    L.append(rng.choice([f"npm http fetch GET 200 https://registry.npmjs.org/{p}/-/{p}-{rng.randint(1,4)}.{rng.randint(0,20)}.{rng.randint(0,9)}.tgz {rng.randint(3,300)}ms (cache miss)",
                         f"npm sill reify mark retired [ 'node_modules/{p}' ]", f"npm timing reifyNode:node_modules/{p} Completed in {rng.randint(1,90)}ms",
                         f"npm WARN deprecated {p}@{rng.randint(0,3)}.{rng.randint(0,9)}.0: this version is no longer supported"]))
L += ["added 287 packages in 9s", "##[endgroup]", "##[group]Run npm test"]
r = subprocess.run(["npm", "test"], cwd="/home/bench/work/express", capture_output=True, text=True)
L += (r.stdout + r.stderr).splitlines()
L += [f"##[error]Process completed with exit code {r.returncode or 1}.", "##[group]Post job cleanup.", "Cleaning up orphan processes", "##[endgroup]"]
open("/home/bench/work/ci/ci.log", "w").write("\n".join(ts() + x for x in L) + "\n")
