#!/usr/bin/env bash
# Engine Wake Observability — A2 (blink) capture launcher.
# Sets up EM_INSTR_FD redirection and starts the GUI; user drives markers
# and clicks by hand. See plan: docs/superpowers/plans/2026-05-03-engine-wake-observability.md
set -euo pipefail

LOG=/tmp/em_instr.blink.log
: > "$LOG"

# fd 9 = log file
exec 9>>"$LOG"

cat <<INSTR
=========================================
A2 BLINK CAPTURE — manual procedure

1. After the GUI window appears, navigate to the runtime test panel
   (crates/emtest) that exposes TextField widgets.
2. Position so the chosen TextField is fully visible. Do NOT move the
   mouse after this point until you click the field.
3. Send open marker:
       kill -USR1 \$(pgrep -f 'target/release/eaglemode')
4. Wait ~5 seconds.
5. Single click into the TextField. Then DO NOT TYPE OR MOVE THE MOUSE.
6. Hold for ~60 seconds. Cursor should visibly blink IF the fix is
   working — this is what we are testing.
7. Send close marker:
       kill -USR1 \$(pgrep -f 'target/release/eaglemode')
8. Quit the GUI normally.

Log will be written to: $LOG
=========================================
INSTR

EM_INSTR_FD=9 cargo run -p eaglemode --release
