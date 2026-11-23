' Starts run_v3.sh in a hidden window, detached from any terminal. Used by the Windows scheduled task "ojasbench-v3".
Set sh = CreateObject("WScript.Shell")
sh.Run """C:\Program Files\Git\usr\bin\bash.exe"" -c ""cd /d/token-saver-bench/bench/v2 && /usr/bin/bash ./run_v3.sh >> ../results/v3-runner.out 2>&1""", 0, False
