#!/bin/sh
# Builds Chord from this checkout, puts it in /Applications and restarts the installed app.
set -e
cd "$(dirname "$0")/.."
npm run tauri build -- --bundles app
rm -rf /Applications/Chord.app
cp -R target/release/bundle/macos/Chord.app /Applications/
# Quitting Chord closes the terminal that runs this script, so the restart has to outlive it.
nohup sh -c '
  binary=/Applications/Chord.app/Contents/MacOS/
  pgrep -qf $binary && osascript -e "tell application \"/Applications/Chord.app\" to quit"
  while pgrep -qf $binary; do sleep 0.5; done
  open /Applications/Chord.app
' >/dev/null 2>&1 &
