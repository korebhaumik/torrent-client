# Torrent Desktop

A standalone macOS and Windows torrent client built with Rust, egui, and librqbit. No browser or Node.js runtime required.

## Features

- Add magnet links, infohashes, or `.torrent` files, including drag and drop.
- Download, seed, pause, resume, and manage files with session recovery.
- Search Pirate Bay, YTS, TV shows, and FitGirl.
- View transfer statistics, filter your library, and switch between dark and light themes.

## Build and run

### macOS (Apple Silicon)

Requires macOS and Rust. The build script also uses Apple's Swift tools to generate icons when needed.

```sh
./scripts/build-macos.sh release
open dist/Torrent.app
```

The current build targets Apple Silicon. The app is locally signed, but not notarized.

### Windows (x64)

Build on Windows with Rust's `x86_64-pc-windows-msvc` toolchain and Visual Studio
Build Tools with **Desktop development with C++** (including the Windows SDK).
From the repository root in PowerShell:

```powershell
rustup target add x86_64-pc-windows-msvc
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\build-windows.ps1 release
```

This produces `dist/Torrent-2.1.0-windows-x64.zip` and its `.zip.sha256` checksum.
Extract the ZIP and run `Torrent.exe` on Windows 10/11 x64. The executable embeds
its icon and version information and links the C runtime statically. No installer
or separate Visual C++ runtime is required. The Windows build is currently unsigned.

The [desktop workflow](.github/workflows/build.yml) tests and packages Windows x64
and macOS Apple Silicon on pushes to `main`/`codex/**`, pull requests, or a manual
run. Download the ZIPs and checksums from the workflow's artifacts. This workflow
does not publish releases. See [Windows release checks](docs/windows.md) before
distributing a build.

## Shortcuts

**⌘K** Add torrent · **⌘O** Choose file · **⌘F** Search · **⌘,** Preferences · **⌘Return** Submit · **Esc** Close

On Windows, use **Ctrl** in place of **⌘**. Shortcut labels adapt automatically.

## Storage

Downloads default to `~/Downloads/Torrent`; change the folder in Preferences.
Settings and session data live in `~/Library/Application Support/Torrent-Desktop/`.
On Windows they live in `%LOCALAPPDATA%\Torrent Desktop\data\`.
Preferences also supports importing a legacy WebTorrent `.state.json` session.

## Development

```sh
./scripts/cargo.sh run
./scripts/cargo.sh test -- --test-threads=1
```

On Windows use `cargo run` and `cargo test --locked -- --test-threads=1` directly.
Set `TORRENT_DATA_DIR` to an isolated folder when testing the UI on either platform.

See [AGENTS.md](AGENTS.md) for architecture and contribution guidelines,
[search verification](postman/simulations/discovery.context.md) for HTTP checks,
and the [optimization report](docs/optimization.md) for measurements and regression coverage.
