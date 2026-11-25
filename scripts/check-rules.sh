#!/bin/sh

status=0
files=$(git ls-files --cached --others --exclude-standard)

if grep -rEn '#[0-9a-fA-F]{3,8}\b' src/app src/features --include='*.ts' --include='*.tsx' --include='*.css' 2>/dev/null; then
  echo "Raw hex colour above. Use a variable from src/ui/tokens.css."
  status=1
fi

if grep -rEn "from ['\"][^'\"]*[./]ios/" src/features --exclude-dir=ios 2>/dev/null; then
  echo "A core feature imports from features/ios above."
  status=1
fi

if grep -rEn 'localStorage|sessionStorage' src --include='*.ts' --include='*.tsx' 2>/dev/null; then
  echo "Browser storage above. Saved state goes through Rust into state.json."
  status=1
fi

if grep -En '^[[:space:]]*(tauri[a-z0-9_-]*|chord-ios)[[:space:]]*=|dependencies\.(tauri|chord-ios)' crates/chord-core/Cargo.toml; then
  echo "chord-core depends on tauri or chord-ios above."
  status=1
fi

if echo "$files" | grep -iE '(^|/)(utils|helpers|common|misc|shared)(/|\.[^/]*$)'; then
  echo "A file or folder above has a catch-all name. Name it after the domain concept it holds."
  status=1
fi

if echo "$files" | grep -v '^scripts/check-rules.sh$' | xargs grep -InE '/Users/|/home/[a-z]' 2>/dev/null; then
  echo "A home folder path above. Code and fixtures must run on a stranger's machine."
  status=1
fi

if echo "$files" | grep -E '\.(ts|tsx|css|rs|swift)$' | grep -v '^src/ipc/bindings.ts$' | xargs grep -nE '^[[:space:]]*(//|/\*|\{/\*)' 2>/dev/null \
  || echo "$files" | grep -E '\.sh$' | xargs grep -nE '^[[:space:]]*#([^!]|$)' 2>/dev/null; then
  echo "A comment above. Put what it says into a name, a type, a test or the commit message."
  status=1
fi

exit $status
