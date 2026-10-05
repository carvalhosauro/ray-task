#!/bin/sh
# Adds ray-task to the Linux application menu (current user only).
# Run from the extracted release archive: ./install-desktop.sh
# If the archive's ray-task binary is next to this script, it is copied to ~/.local/bin.
set -eu
here=$(cd "$(dirname "$0")" && pwd)
dest=${XDG_DATA_HOME:-$HOME/.local/share}

# Desktop sessions often lack ~/.cargo/bin on PATH, so the menu entry uses an absolute path.
if [ -x "$here/ray-task" ]; then
  bin="$HOME/.local/bin/ray-task"
  install -Dm755 "$here/ray-task" "$bin"
elif command -v ray-task >/dev/null 2>&1; then
  bin=$(command -v ray-task)
elif [ -x "${CARGO_HOME:-$HOME/.cargo}/bin/ray-task" ]; then
  bin="${CARGO_HOME:-$HOME/.cargo}/bin/ray-task"
else
  echo "ray-task binary not found. Run this script from the extracted release archive," >&2
  echo "or install ray-task first (see README → Install)." >&2
  exit 1
fi

# dist copies the icon dir to the archive root as ./icon; in the repo it lives under crates/.
if [ -d "$here/icon" ]; then
  icons="$here/icon"
else
  icons="$here/../crates/app-slint/assets/icon"
fi

for n in 16 32 48 64 128 256 512; do
  install -Dm644 "$icons/ray-task-$n.png" "$dest/icons/hicolor/${n}x${n}/apps/ray-task.png"
done
mkdir -p "$dest/applications"
awk -v exec="Exec=\"$bin\"" '/^Exec=/ { print exec; next } { print }' "$here/ray-task.desktop" \
  >"$dest/applications/ray-task.desktop"

command -v update-desktop-database >/dev/null 2>&1 && update-desktop-database "$dest/applications" || true
echo "ray-task added to your application menu (runs $bin)."
