#!/bin/sh
# Runs install-desktop.sh from a copy in a path with spaces, from another cwd,
# into throwaway HOME/XDG_DATA_HOME dirs, and checks every installed file.
set -eu
repo=$(cd "$(dirname "$0")/.." && pwd)
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
fail=0

# Simulate the extracted release archive layout: binary, script, .desktop, icon/ side by side.
pkg="$work/ray-task v0.2 extracted"
mkdir -p "$pkg/icon"
cp "$repo/packaging/install-desktop.sh" "$repo/packaging/ray-task.desktop" "$pkg/"
cp "$repo"/crates/app-slint/assets/icon/ray-task-*.png "$pkg/icon/"
printf '#!/bin/sh\n' >"$pkg/ray-task"
chmod +x "$pkg/ray-task"

# Case 1: archive ships the binary → copied to ~/.local/bin, menu entry points at it.
home="$work/home dir"
data="$work/data home"
mkdir -p "$home"
(cd / && env -u CARGO_HOME HOME="$home" XDG_DATA_HOME="$data" PATH=/usr/bin:/bin sh "$pkg/install-desktop.sh") >/dev/null

for f in "applications/ray-task.desktop" \
         "icons/hicolor/16x16/apps/ray-task.png" \
         "icons/hicolor/128x128/apps/ray-task.png" \
         "icons/hicolor/512x512/apps/ray-task.png"; do
  [ -f "$data/$f" ] || { echo "FAIL: missing $f"; fail=1; }
done
[ -x "$home/.local/bin/ray-task" ] || { echo "FAIL: binary not installed to ~/.local/bin"; fail=1; }
grep -qx "Exec=\"$home/.local/bin/ray-task\"" "$data/applications/ray-task.desktop" ||
  { echo "FAIL: Exec is not the absolute installed path:"; grep '^Exec=' "$data/applications/ray-task.desktop"; fail=1; }

# Case 2: no binary anywhere → non-zero exit, clear message, no menu entry.
rm "$pkg/ray-task"
data2="$work/data2"
home2="$work/home2"
mkdir -p "$home2"
if out=$(cd / && env -u CARGO_HOME HOME="$home2" XDG_DATA_HOME="$data2" PATH=/usr/bin:/bin sh "$pkg/install-desktop.sh" 2>&1); then
  echo "FAIL: succeeded without a ray-task binary"; fail=1
fi
case $out in *"ray-task binary not found"*) ;; *) echo "FAIL: unclear message: $out"; fail=1 ;; esac
[ ! -e "$data2/applications/ray-task.desktop" ] || { echo "FAIL: menu entry created without binary"; fail=1; }

[ "$fail" -eq 0 ] && echo "install-desktop ok"
exit "$fail"
