' Starts run_v2.sh in a hidden window, detached from any terminal. Used by the Windows scheduled task "ojasbench-v2".
Set sh = CreateObject("WScript.Shell")
sh.Run """C:\Program Files\Git\usr\bin\bash.exe"" -c ""cd /d/token-saver-bench/bench/v2 && /usr/bin/bash ./run_v2.sh >> ../results/v2/runner.out 2>&1""", 0, False
