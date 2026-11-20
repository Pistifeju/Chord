#!/bin/sh
# Runs every check even after one fails, prints one exit line per step, and keeps the output in a log of its own.

log=$(mktemp -t chord-check)
out=$(mktemp -t chord-step)
status=0

step() {
  name=$1
  shift
  start=$(date +%s)
  "$@" > "$out" 2>&1
  code=$?
  echo "step=$name exit=$code secs=$(($(date +%s) - start))"
  { echo "=== $name"; cat "$out"; } >> "$log"
  if [ "$code" -ne 0 ]; then
    status=1
    tail -n 20 "$out" | sed 's/^/  /'
  fi
}

step tsc npx tsc --noEmit
step tsc-test npx tsc --noEmit -p tsconfig.test.json
step rules sh scripts/check-rules.sh
step test node --test "src/**/*.test.ts"
step fmt cargo fmt --all --check
# The Rust steps take minutes after a rebuild, so a local change outside Rust skips them. CI and a clean tree run them.
changes=$(git status --porcelain)
if [ -z "$CI" ] && [ -n "$changes" ] && ! printf '%s\n' "$changes" | grep -qE 'crates/|src-tauri/|Cargo\.(toml|lock)|rust-toolchain|\.cargo/'; then
  echo "step=clippy exit=skipped"
  echo "step=cargo-test exit=skipped"
else
  step clippy cargo clippy --workspace --all-targets --locked -- -D warnings
  step cargo-test cargo test --workspace --all-targets --locked
fi

rm -f "$out"
echo "log=$log"
exit $status
