#!/bin/sh
# Without a folder: prints every source folder with its file count. With folders: prints their source files, each with its first line, which says what the file holds.

sources() {
  find "$@" \( -name '*.ts*' -o -name '*.rs' \) -not -path '*/target/*' -not -path '*/node_modules/*'
}

if [ $# -eq 0 ]; then
  sources src crates src-tauri | sed 's|/[^/]*$||' | sort | uniq -c
else
  sources "$@" | sort | xargs awk 'FNR==1{print FILENAME": "$0; nextfile}'
fi
