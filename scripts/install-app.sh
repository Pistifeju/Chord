#!/bin/sh
set -e
cd "$(dirname "$0")/.."
npm run tauri build -- --bundles app
rm -rf /Applications/Chord.app
cp -R target/release/bundle/macos/Chord.app /Applications/
nohup sh -c '
  binary=/Applications/Chord.app/Contents/MacOS/
  pgrep -qf $binary && osascript -e "tell application \"/Applications/Chord.app\" to quit"
  while pgrep -qf $binary; do sleep 0.5; done
  open /Applications/Chord.app
' >/dev/null 2>&1 &
