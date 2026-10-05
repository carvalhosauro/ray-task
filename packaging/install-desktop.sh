#!/bin/sh
# Adds ray-task to the Linux application menu (current user only).
# Run from the extracted release archive: ./install-desktop.sh
# The binary itself is installed by the shell installer or copied by you onto PATH.
set -eu
here=$(cd "$(dirname "$0")" && pwd)
dest=${XDG_DATA_HOME:-$HOME/.local/share}

# dist copies the icon dir to the archive root as ./icon; in the repo it lives under crates/.
if [ -d "$here/icon" ]; then
  icons="$here/icon"
else
  icons="$here/../crates/app-slint/assets/icon"
fi

for n in 16 32 48 64 128 256 512; do
  install -Dm644 "$icons/ray-task-$n.png" "$dest/icons/hicolor/${n}x${n}/apps/ray-task.png"
done
install -Dm644 "$here/ray-task.desktop" "$dest/applications/ray-task.desktop"

command -v update-desktop-database >/dev/null 2>&1 && update-desktop-database "$dest/applications" || true
echo "ray-task added to your application menu."
