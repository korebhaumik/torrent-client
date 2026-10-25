# Torrent Desktop

Standalone macOS torrent client written in Rust. The desktop UI and torrent engine run in one native process, using egui/eframe and librqbit. The packaged app requires no Node.js, Next.js, browser, or local HTTP server.

## Open the app

Open **`dist/Torrent.app`** in Finder. You can move it to Applications or another folder. The build produced here targets **Apple Silicon (arm64)**. It has a local ad hoc code signature; it is not Apple notarized for distribution to other Macs.

The app recreates the original dark/light colors, 240-point sidebar, Library and Discover views, live statistics, torrent card grids, status filters, dialogs, and detail drawers. The interface is drawn natively in Rust; font rendering and animations can differ from the former CSS interface.

Supported actions:

- Add a magnet URI, a 40-character v1 infohash, or a `.torrent` file (up to 10 MB).
- Drop a `.torrent` file onto the window or use a native file picker.
- Download, seed, pause, resume, retry, remove, and optionally delete downloaded files.
- Filter by name, infohash, and status; inspect progress, peers, speeds, ETA, ratio, identifiers, and files.
- Copy identifiers, open files in their macOS application, and reveal them in Finder.
- Search Pirate Bay, YTS, TV shows through TVMaze/EZTV, and FitGirl, with source filters and result details.
- Switch themes and choose a download folder in Preferences.
- Restore saved torrents and paused state after restarting.

Keyboard shortcuts: **⌘K** toggles Add Torrent, **⌘O** chooses a torrent file, **⌘F** focuses search, **⌘,** opens Preferences, **⌘Return** submits the Add form, and **Esc** closes a dialog or drawer.

## Interface refinements in 2.1

The original magnet logo is retained. The native UI now uses consistent control sizes, rounded cards, clearer typography, accessible button labels, keyboard focus rings, hover and disabled states, and refined Add, Remove, Preferences, and detail views. Add form validation stays beside the relevant field; Preferences scrolls on smaller windows.

## Existing downloads and state

On first launch from this project, the app recognizes the original `downloads/.state.json`, imports its saved magnets, and uses the existing download directory. It verifies existing file pieces instead of starting completed files from zero. The old state file is left intact. The saved magnet list was empty at migration time; existing media files remain in `downloads/`.

For a separate installation, new downloads default to `~/Downloads/Torrent`. Preferences → **Import WebTorrent Session…** imports an old `.state.json` and uses the folder containing it for those torrents. Existing media files without a corresponding saved magnet or torrent cannot be reconstructed into torrent entries automatically.

The app stores settings, torrent records, and cached torrent metadata under `~/Library/Application Support/Torrent-Desktop/`. Each torrent remembers its own destination; changing Preferences affects newly added torrents. Multi-file torrents retain their named parent directory. Only one app instance can use a session at a time.

The Rust engine supports BitTorrent v1, TCP peers, DHT, and HTTP/UDP trackers. Browser-only WebRTC swarms are not supported. Discovery uses the same public services as the original app; their availability and results depend on those services and the network. Native file opening replaces the former browser file endpoint.

## Build and develop

The locally installed compiler lives in `.tools/`; it did not change the system PATH. If Rust is already installed, the scripts also work with the normal `cargo` command.

```sh
./scripts/build-macos.sh          # optimized, signed dist/Torrent.app
./scripts/build-macos.sh debug    # development app bundle
./scripts/cargo.sh run
./scripts/cargo.sh test -- --test-threads=1
./scripts/cargo.sh clippy --all-targets -- -D warnings
```

App code is in `native/src/`. `Cargo.lock` pins dependency versions. The small Swift script only generates the macOS icon during a build; the application itself is Rust.

Development overrides:

- `TORRENT_DATA_DIR`: isolated app-state directory.
- `TORRENT_SEARCH_BASE`: local HTTP fixture base, e.g. `http://127.0.0.1:4900/search`.
- `RUST_LOG`: logging filter.

The binary also supports `--search QUERY --source all|piratebay|yts|tv|fitgirl` for HTTP diagnostics without launching a window.

## Validation

`native/tests/core.rs` exercises real loopback BitTorrent transfer with content verification, pause/resume and restart recovery, cached metadata recovery after the input torrent file is removed, preservation versus deletion of downloaded files, preservation of unrelated files, multi-file destination layout, invalid input, legacy import, and exclusive session locking.

Search checks run the actual Rust HTTP client through a local Postman simulation:

```sh
postman simulation run postman/simulations/discovery.sim.yaml --port 4900 --output ndjson
# In another terminal:
python3 scripts/check-search.py
```

The fixtures model the provider response shapes consumed by the Rust search adapters; they are development data, not live provider results. An owner-mock lookup was skipped because Postman is signed out. The tests verify all four adapters, individual source selection, sorting, sizes, partial outages, HTTP 503, malformed upstream responses, and short queries. Inspect the simulation request log to verify the production client paths ran. No mocks were uploaded or deployed.

The macOS window is also checked interactively for layout, dialogs, discovery, themes, result details, and restored torrent cards. A successful local test does not guarantee that every external index or public torrent swarm is available.
