#!/bin/sh

sources() {
  find "$@" \( -name '*.ts*' -o -name '*.rs' \) -not -path '*/target/*' -not -path '*/node_modules/*'
}

if [ $# -eq 0 ]; then
  sources src crates src-tauri | sed 's|/[^/]*$||' | sort | uniq -c
else
  sources "$@" | sort
fi
