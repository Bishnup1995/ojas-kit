# Sourced as root inside a container. Copies the harness files (mounted read-only under /root, which user bench
# cannot traverse) to a bench-readable temp dir, for setup or grading only. stage_in / stage_out.
stage_in() {
  rm -rf /tmp/v2s && mkdir -p /tmp/v2s/layer2 /bench
  cp -r /root/v2/fixtures.py /root/v2/tasks_v2.sh /root/v2/graders /root/v2/validate /tmp/v2s/
  cp /root/layer2/tasks.sh /tmp/v2s/layer2/tasks.sh && cp /root/layer2/make_log.py /bench/make_log.py
  chown -R bench:bench /tmp/v2s /bench && chmod -R a+rX /tmp/v2s /bench
}
stage_out() { rm -rf /tmp/v2s /bench; }
asbench() { runuser -u bench -- env -i HOME=/home/bench USER=bench LOGNAME=bench SHELL=/bin/bash PATH="$PATH" TERM=dumb LANG=C.UTF-8 \
  DISABLE_AUTOUPDATER=1 DISABLE_TELEMETRY=1 CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC=1 ARM="$ARM" V2=/tmp/v2s "$@"; }
