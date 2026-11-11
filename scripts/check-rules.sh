#!/bin/sh
# Fails on a raw hex colour in a component and on a core feature that imports iOS code.

status=0

if grep -rEn '#[0-9a-fA-F]{3,8}\b' src/app src/features --include='*.ts' --include='*.tsx' --include='*.css' 2>/dev/null; then
  echo "Raw hex colour above. Use a variable from src/ui/tokens.css."
  status=1
fi

if grep -rEn "from ['\"][^'\"]*[./]ios/" src/features --exclude-dir=ios 2>/dev/null; then
  echo "A core feature imports from features/ios above."
  status=1
fi

exit $status
