# Recording the demo GIF

Target: `docs/assets/demo.gif`, ~10 s, 820 px wide, keyboard only, no mouse cursor visible.
Once it exists, uncomment the demo block at the top of the root `README.md`.

## Script

1. Start with a clean database: `XDG_DATA_HOME=$(mktemp -d) cargo run -p ray-task --release`
2. Create 3–4 realistic tasks first (e.g. "Renew passport", "Review PR #42", "Buy coffee"), then start recording.
3. `Ctrl+N` → type `Write release notes` → `Enter`
4. `Ctrl+D` → `A` (tomorrow)
5. `Ctrl+T` → type `work` → `Enter`
6. `↑` to another task → `Ctrl+Enter` (complete it)
7. `Ctrl+2` (Upcoming) → `Ctrl+1` (Today)
8. Stop recording.

## Record and convert (Fedora, GNOME Wayland)

- Record: `Ctrl+Shift+Alt+R` (GNOME screencast) or the screenshot UI's video mode; it saves a `.webm` in `~/Videos/Screencasts`.
- Convert:
  ```bash
  sudo dnf install ffmpeg gifski
  mkdir -p /tmp/frames
  ffmpeg -i ~/Videos/Screencasts/<file>.webm -vf "fps=20,scale=820:-1" /tmp/frames/f%04d.png
  gifski --fps 20 --width 820 --quality 90 -o docs/assets/demo.gif /tmp/frames/f*.png
  ```
- Keep it under 5 MB (GitHub renders larger GIFs slowly). Lower `--quality` or `fps` if needed.

## Social preview

`social-preview.png` is rendered from `social-preview.html` at 1280×640. Serve the repo root
(`python3 -m http.server`), open the page at that viewport size and screenshot it.
Upload the PNG in GitHub Settings → General → Social preview.
