#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
TARGET_DIR="$ROOT_DIR/src-tauri/target/debug"
SUBDIRS=(".fingerprint" "build" "deps" "incremental")

for subdir in "${SUBDIRS[@]}"; do
  dir="$TARGET_DIR/$subdir"
  if [ ! -d "$dir" ]; then
    continue
  fi

  mapfile -t items < <(ls -1t "$dir")
  for index in "${!items[@]}"; do
    if [ "$index" -ge 2 ]; then
      rm -rf "$dir/${items[$index]}"
    fi
  done
done
