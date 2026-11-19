#!/bin/sh
# Runs every check even after one fails, prints one exit line per step, and keeps the output in a log of its own.

log=$(mktemp -t chord-check)
status=0

step() {
  name=$1
  shift
  echo "=== $name" >> "$log"
  "$@" >> "$log" 2>&1
  code=$?
  echo "step=$name exit=$code"
  [ "$code" -ne 0 ] && status=1
}

step tsc npx tsc --noEmit
step rules sh scripts/check-rules.sh
step test node --test "src/**/*.test.ts"
step fmt cargo fmt --all --check
step clippy cargo clippy --workspace --all-targets --locked -- -D warnings
step cargo-test cargo test --workspace --locked

echo "log=$log"
exit $status
