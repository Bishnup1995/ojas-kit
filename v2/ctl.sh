#!/usr/bin/env bash
# Control the detached v2 runner.  bench/v2/ctl.sh status|start|stop|kill|analyze
cd "$(dirname "$0")"; OUT=../results/v2
case "${1:-status}" in
  status) tail -n 5 $OUT/progress.log 2>/dev/null; echo "--- pauses:"; tail -n 3 $OUT/pauses.log 2>/dev/null
          pid=$(cat $OUT/.runner.lock/pid 2>/dev/null); if [ -n "$pid" ] && kill -0 "$pid" 2>/dev/null; then echo "runner RUNNING (pid $pid)"; else echo "runner NOT running"; fi
          docker ps --filter name=ojasbench-v2-run --format 'container: {{.Names}} {{.Status}}' ;;
  start)  schtasks //Run //TN ojasbench-v2 ;;
  stop)   touch $OUT/STOP; echo "STOP requested: the runner exits after the current run" ;;
  kill)   pid=$(cat $OUT/.runner.lock/pid 2>/dev/null); [ -n "$pid" ] && kill "$pid"; docker rm -f ojasbench-v2-run; rm -rf $OUT/.runner.lock; echo "killed (the interrupted run is redone on resume)" ;;
  analyze) python analyze_v2.py ;;
esac
