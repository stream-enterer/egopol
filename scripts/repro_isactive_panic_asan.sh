#!/usr/bin/env bash
set -euo pipefail

echo "=== REPRO INSTRUCTIONS (ASan) ===" >&2
echo "1. File manager opens by default" >&2
echo "2. Click into a directory entry (zoom in)" >&2
echo "3. Click out (zoom out)" >&2
echo "4. Crash should appear; ASan may report BEFORE the panic" >&2

# halt_on_error=0 keeps the process alive after the first ASan report so we
# capture every heap issue up to and including the panic.
# detect_leaks=0 silences leak reports at exit (we only care about UAF/UB).
ASAN_OPTIONS="detect_leaks=0:symbolize=1:halt_on_error=0:abort_on_error=0:print_stacktrace=1" \
RUSTFLAGS="-Zsanitizer=address -Cforce-frame-pointers=yes -Clink-arg=-Wl,--export-dynamic" \
RUST_BACKTRACE=1 \
exec cargo +nightly run --target x86_64-unknown-linux-gnu -p eaglemode 2>&1 | tee target/asan-run.log
