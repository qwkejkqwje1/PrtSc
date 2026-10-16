set -e
source /data/qa08/wine-env.sh
cd /data/prtsc08/PrtSc-0.7.0
wine64 target/x86_64-pc-windows-gnu/release/PrtSc.exe > /data/qa08/app.log 2>&1 &
sleep 2
p=/data/qa08
wine64 $p/win_actions_probe.exe command 11
sleep 1
wine64 $p/win_revision07_probe.exe focus
wine64 $p/win_revision07_probe.exe press 3 75
wine64 $p/win_revision07_probe.exe read
wine64 $p/win_revision08_probe.exe snapshot-up
wine64 $p/win_revision07_probe.exe read
wine64 $p/win_revision07_probe.exe press 3 75
wine64 $p/win_revision08_probe.exe snapshot-message-up
wine64 $p/win_revision07_probe.exe read
wine64 $p/win_revision07_probe.exe press 3 75
wine64 $p/win_revision07_probe.exe save
wine64 $p/win_actions_probe.exe command 11
sleep 1
wine64 $p/win_revision07_probe.exe focus
wine64 $p/win_revision07_probe.exe press 0 44
wine64 $p/win_revision07_probe.exe read
DISPLAY=:99 import -window root $p/printscreen-recorded.png
wine64 $p/win_revision07_probe.exe save
wine64 $p/win_revision07_probe.exe press 0 44
wine64 $p/win_revision06_probe.exe windows
wine64 $p/win_gui_probe.exe key 27
wine64 $p/win_actions_probe.exe command 11
sleep 1
wine64 $p/win_revision07_probe.exe focus
wine64 $p/win_revision07_probe.exe press 0 44
wine64 $p/win_revision07_probe.exe read
wine64 $p/win_revision07_probe.exe press 6 44
wine64 $p/win_revision07_probe.exe read
wine64 $p/win_revision08_probe.exe snapshot-up
wine64 $p/win_revision07_probe.exe press 0 9
wine64 $p/win_revision06_probe.exe cancelsettings
wine64 $p/win_revision07_probe.exe press 0 44
wine64 $p/win_revision06_probe.exe windows
wine64 $p/win_gui_probe.exe key 27
