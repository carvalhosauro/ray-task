#!/bin/sh
# Runs install-desktop.sh from a copy in a path with spaces, from another cwd,
# into a throwaway XDG_DATA_HOME, and checks every installed file.
set -eu
repo=$(cd "$(dirname "$0")/.." && pwd)
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

# Simulate the extracted release archive layout: script, .desktop, icon/ side by side.
pkg="$work/ray-task v0.2 extracted"
mkdir -p "$pkg/icon"
cp "$repo/packaging/install-desktop.sh" "$repo/packaging/ray-task.desktop" "$pkg/"
cp "$repo"/crates/app-slint/assets/icon/ray-task-*.png "$pkg/icon/"

data="$work/data home"
(cd / && XDG_DATA_HOME="$data" sh "$pkg/install-desktop.sh") >/dev/null

fail=0
for f in "applications/ray-task.desktop" \
         "icons/hicolor/16x16/apps/ray-task.png" \
         "icons/hicolor/128x128/apps/ray-task.png" \
         "icons/hicolor/512x512/apps/ray-task.png"; do
  [ -f "$data/$f" ] || { echo "FAIL: missing $f"; fail=1; }
done
[ "$fail" -eq 0 ] && echo "install-desktop ok"
exit "$fail"
