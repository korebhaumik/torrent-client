# Torrent Desktop

A standalone macOS torrent client built with Rust, egui, and librqbit. No browser or Node.js runtime required.

## Features

- Add magnet links, infohashes, or `.torrent` files, including drag and drop.
- Download, seed, pause, resume, and manage files with session recovery.
- Search Pirate Bay, YTS, TV shows, and FitGirl.
- View transfer statistics, filter your library, and switch between dark and light themes.

## Build and run

Requires macOS and Rust. The build script also uses Apple's Swift tools to generate icons when needed.

```sh
./scripts/build-macos.sh release
open dist/Torrent.app
```

The current build targets Apple Silicon. The app is locally signed, but not notarized.

## Shortcuts

**⌘K** Add torrent · **⌘O** Choose file · **⌘F** Search · **⌘,** Preferences · **⌘Return** Submit · **Esc** Close

## Storage

Downloads default to `~/Downloads/Torrent`; change the folder in Preferences.
Settings and session data live in `~/Library/Application Support/Torrent-Desktop/`.
Preferences also supports importing a legacy WebTorrent `.state.json` session.

## Development

```sh
./scripts/cargo.sh run
./scripts/cargo.sh test -- --test-threads=1
```

See [AGENTS.md](AGENTS.md) for architecture and contribution guidelines,
[search verification](postman/simulations/discovery.context.md) for HTTP checks,
and the [optimization report](docs/optimization.md) for measurements and regression coverage.
