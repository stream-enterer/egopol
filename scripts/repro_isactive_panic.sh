#!/usr/bin/env bash
set -euo pipefail

cargo build --bin eaglemode 2>&1 | tee target/repro-build.log

echo "=== REPRO INSTRUCTIONS ===" >&2
echo "1. File manager opens by default" >&2
echo "2. Click into a directory entry (zoom in)" >&2
echo "3. Click out (zoom out)" >&2
echo "4. Crash should appear" >&2

RUST_BACKTRACE=1 exec target/debug/eaglemode 2>&1 | tee target/repro-run.log
