import os, random
N = int(os.environ.get("LOGLINES", "6000"))
random.seed(11)
L = []
for i in range(N):
    L.append(f"2026-11-20T{9 + i // 3600:02d}:{i // 60 % 60:02d}:{i % 60:02d}Z INFO  worker-{random.randint(1, 8)} handled request id={random.randint(10**5, 10**6)} status=200 took={random.randint(5, 90)}ms")
for k in random.sample(range(N), 60):
    L[k] = L[k].replace("INFO ", "WARN ").replace("status=200", "status=200 slow_query=true")
for k in random.sample(range(N), 25):
    L[k] = L[k].replace("INFO ", "ERROR").replace("status=200", "status=502 upstream timeout (retrying, attempt 1/3)")
L[4210] = "2026-11-20T10:10:10Z FATAL worker-3 db pool exhausted: cannot acquire connection after 30s (pool=20 in_use=20 waiting=147) incident=INC-7741"
open("/home/bench/work/logs/app.log", "w").write("\n".join(L) + "\n")
