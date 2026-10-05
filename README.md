<p align="center">
  <img src="crates/app-slint/assets/icon/ray-task-128.png" alt="" width="96" height="96">
</p>

<h1 align="center">5,000 tasks. 40 MB. Zero accounts.</h1>

<p align="center">
  <b>ray-task</b> is a keyboard-first TODO app for your desktop.<br>
  Local SQLite, no account, no subscription. Written in Rust.
</p>

<p align="center">
  <a href="https://github.com/carvalhosauro/ray-task/actions/workflows/ci.yml"><img alt="CI" src="https://github.com/carvalhosauro/ray-task/actions/workflows/ci.yml/badge.svg"></a>
  <a href="https://github.com/carvalhosauro/ray-task/releases/latest"><img alt="Latest release" src="https://img.shields.io/github/v/release/carvalhosauro/ray-task"></a>
  <a href="LICENSE"><img alt="MIT license" src="https://img.shields.io/badge/license-MIT-blue"></a>
  <img alt="MSRV 1.92" src="https://img.shields.io/badge/MSRV-1.92-orange">
  <a href="https://slint.dev"><img alt="Made with Slint" src="https://raw.githubusercontent.com/slint-ui/slint/master/logo/MadeWithSlint-logo-whitebg.png" height="20"></a>
</p>

<!-- Demo GIF goes here once recorded (see docs/assets/README.md):
<p align="center">
  <img src="docs/assets/demo.gif" alt="Adding a task, setting a due date and a tag, completing it and switching views, keyboard only" width="820">
</p>
-->

<p align="center"><a href="#install"><b>Install in one line →</b></a></p>

```sh
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/carvalhosauro/ray-task/releases/latest/download/ray-task-installer.sh | sh
```

---

## The problem

Your TODO list is a few kilobytes of text. To edit it, you log in, wait for a sync, and hand a browser engine hundreds of megabytes of RAM. Then you pay monthly for the privilege.

ray-task keeps the list in one SQLite file on your disk and opens before you reach for the mouse.

## The numbers

Measured with **5,000 tasks** in the database (RAM measured on Linux with the default software renderer):

| RAM | Switching views | Cold start (open + load) |
|:---:|:---:|:---:|
| **< 40 MB** | **< 2 ms** | **< 150 ms** |

Every number is a test you can run: [`crates/core/tests/perf.rs`](crates/core/tests/perf.rs) and the RAM procedure in [`docs/manual-checklist.md`](docs/manual-checklist.md).

## How it compares

|  | ray-task | Todoist | TickTick | Things 3 | Microsoft To Do |
|---|:---:|:---:|:---:|:---:|:---:|
| Account required | **No** | Yes | Yes | No | Yes |
| Subscription for full features | **No** | Yes | Yes | No (paid app) | No |
| Your data stays on your disk | **Yes** | No | No | Yes | No |
| Open source | **Yes** | No | No | No | No |
| Runs on Linux | **Yes** | Yes | Yes | No | No |
| Price | **Free** | Freemium | Freemium | Paid | Free |

## Keyboard first

| Shortcut | Action |
|---|---|
| `Ctrl+N` | New task in the current view |
| `Ctrl+Shift+N` | New project |
| `↑` / `↓` | Move between tasks |
| `Enter` / `Esc` | Open / close |
| `Ctrl+Enter` | Complete / uncomplete |
| `Ctrl+D` | Due date (in the popover: `H` today, `A` tomorrow, `S` next week, `Delete` no date, or type a time) |
| `Ctrl+T` | Tag (`Enter` uses what you typed, `Tab` accepts the suggestion) |
| `Delete` | Delete (with Undo) |
| `Ctrl+Z` | Undo |
| `Ctrl+1` / `2` / `3` | Today / Upcoming / Inbox |
| `Ctrl+4` … `Ctrl+9` | Projects, in sidebar order |
| `Ctrl+F` | Filter the current view |

Right-click a project to rename, recolor or delete it. Click an open tag to remove, rename or delete it.

> The interface is currently in Brazilian Portuguese. Translations are welcome, see [CONTRIBUTING](CONTRIBUTING.md).

## Install

**Linux and macOS**

```sh
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/carvalhosauro/ray-task/releases/latest/download/ray-task-installer.sh | sh
```

**Windows**: download the [`.msi` installer](https://github.com/carvalhosauro/ray-task/releases/latest/download/ray-task-x86_64-pc-windows-msvc.msi), or in PowerShell:

```powershell
powershell -ExecutionPolicy Bypass -c "irm https://github.com/carvalhosauro/ray-task/releases/latest/download/ray-task-installer.ps1 | iex"
```

**From source** (any OS with Rust 1.92+):

```sh
cargo install --git https://github.com/carvalhosauro/ray-task ray-task
```

Prebuilt archives for every platform are on the [releases page](https://github.com/carvalhosauro/ray-task/releases/latest).

<details>
<summary><b>Linux: add ray-task to your application menu</b></summary>

Download and extract the archive for your architecture from the releases page, then run:

```sh
./install-desktop.sh
```

It copies `ray-task` to `~/.local/bin` and adds the menu entry and icons. No root needed.
</details>

<details>
<summary><b>macOS / Windows: "unidentified developer" warning</b></summary>

Binaries are not code-signed yet.

- **macOS:** `xattr -d com.apple.quarantine ~/.cargo/bin/ray-task`, or right-click → Open the first time.
- **Windows:** in the SmartScreen dialog, click *More info* → *Run anyway*.
</details>

## Where your data lives

| | Database | Log |
|---|---|---|
| Linux | `~/.local/share/ray-task/ray-task.db` | `~/.local/state/ray-task/ray-task.log` |
| macOS | `~/Library/Application Support/ray-task/ray-task.db` | same folder |
| Windows | `%APPDATA%\ray-task\ray-task.db` | same folder |

Before a schema migration, ray-task copies the database to `ray-task.db.bak`. Back up that one file and you have backed up everything.

## Build from source

Linux needs Slint's system libraries:

- Fedora: `sudo dnf install fontconfig-devel libxkbcommon-devel wayland-devel`
- Debian/Ubuntu: `sudo apt install libfontconfig1-dev libxkbcommon-dev libwayland-dev`
- Arch: `sudo pacman -S fontconfig libxkbcommon wayland`

```sh
git clone https://github.com/carvalhosauro/ray-task
cd ray-task
cargo run -p ray-task --release
```

ray-task uses Slint's software renderer by default to stay under 40 MB. For the GPU renderer: `SLINT_BACKEND=winit-femtovg cargo run -p ray-task --release` (about 50 MB more).

## Why this exists

My computer freezes when I open too many Electron apps. A TODO list was one of them: a list of text, eating hundreds of megabytes next to my editor and browser.

So I built the TODO app I wanted. It opens instantly, never asks me to log in, never touches the network, and I never take my hands off the keyboard.

If your machine is old, small, or busy, ray-task is for you.

— [@carvalhosauro](https://github.com/carvalhosauro)

## Contributing

Bug reports, ideas and PRs are welcome. Start with [CONTRIBUTING.md](CONTRIBUTING.md); setup takes about five minutes.

## License

[MIT](LICENSE). The Inter font is © The Inter Project Authors under the SIL Open Font License 1.1 (`crates/app-slint/assets/fonts/LICENSE.txt`). Built with [Slint](https://slint.dev) under its royalty-free license.

---

<p align="center">
  If ray-task gave you back some RAM, <b>give it a ⭐</b>. It helps other people find it.<br>
  <a href="https://twitter.com/intent/tweet?text=A%20TODO%20app%20that%20runs%20in%2040%20MB%20of%20RAM%2C%20no%20account%2C%20keyboard-first.&url=https%3A%2F%2Fgithub.com%2Fcarvalhosauro%2Fray-task">Share on X</a> ·
  <a href="https://bsky.app/intent/compose?text=A%20TODO%20app%20that%20runs%20in%2040%20MB%20of%20RAM%2C%20no%20account%2C%20keyboard-first.%20https%3A%2F%2Fgithub.com%2Fcarvalhosauro%2Fray-task">Share on Bluesky</a>
</p>
