#!/bin/sh
# Prints every source file with its first line, which says what the file holds.

find src crates src-tauri \( -name '*.ts*' -o -name '*.rs' \) -not -path '*/target/*' -not -path '*/node_modules/*' \
  -exec awk 'FNR==1{print FILENAME": "$0; nextfile}' {} + | sort
