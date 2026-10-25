# Working on Torrent Desktop

This repository is a standalone Rust macOS application. Preserve its existing UI,
keyboard behavior, accessibility, and torrent/session semantics unless the task
explicitly changes them. Read `README.md` and the relevant source and tests before
editing. Use a feature branch; do not commit or push directly to `main`.

## Architecture and ownership

- `native/src/main.rs`: process startup, native window/icon, and diagnostic CLI.
- `native/src/app.rs`: UI state, navigation, library/discovery, and frame updates.
  `app/dialogs.rs` owns Add, Remove, and Preferences; `app/details.rs` owns drawers.
- `native/src/design.rs`: shared colors, typography, accessible controls, and layout.
  Reuse these controls rather than creating competing visual systems.
- `native/src/engine.rs`: torrent worker, commands, lifecycle, metadata, and recovery.
  UI code sends commands and reads immutable snapshots; it must not own torrent
  handles or hold engine locks while rendering.
- `native/src/search.rs`: HTTP orchestration, mirrors, timeouts, and provider errors.
  `search/parsing.rs` handles pure normalization; `search/cache.rs` bounds cache memory.
- `native/src/model.rs`: shared data and pure presentation calculations.
  `native/src/config.rs`: paths, settings, exclusive session lock, atomic persistence.
- `native/tests/`: integration tests using temporary data and local peers.
  `postman/` and `scripts/check-search.py`: deterministic search HTTP fixtures.

Keep dependencies pointing from UI/engine/search toward model/config, not back into
the UI. Use small concrete functions and types with clear owners. Split modules by
responsibility when they become hard to navigate; avoid speculative frameworks,
generic wrappers, and global mutable services. Restrict visibility to the narrowest
scope that works. Comments should explain invariants or non-obvious decisions.

## Behavior and data contracts

- Keep network, torrent work, and response parsing off the UI thread. Bound network
  responses, timeouts, concurrency, and caches. Reuse clients and compiled regexes.
- Cancel obsolete searches and discard stale responses. Do not clone the entire
  library or file list on each frame. Preserve snapshot consistency.
- Keep saved JSON fields compatible, including serde defaults. Write atomically,
  preserve unreadable originals, and retain per-torrent destinations and metadata.
- Paused torrents, restart recovery, multi-file parent folders, legacy session
  import, and remove-with/without-files are supported contracts, not dead code.
- Delete only files belonging to the selected torrent when explicitly requested.
  Never use real user downloads or production Application Support state in tests.
- Return actionable errors for external data, I/O, and networking. Do not introduce
  `unwrap`/`expect` on these paths. An expectation for a literal regex or static
  invariant must explain that invariant. Avoid silent success on failed writes.
- Preserve native file picking, text clipboard, file opening/reveal, shortcuts,
  both themes, small-window scrolling, accessible labels, and disabled controls.
  Do not remove font fallbacks or accessibility to reduce binary size.

## Dependencies and builds

Prefer the standard library and existing crates. Add dependencies only for a
concrete need, enable only required features, and commit `Cargo.lock`. Inspect
`cargo tree -e features` before claiming a feature is removed: transitive crates
may still enable it. Do not upgrade unrelated dependencies during a refactor.

Keep application runtime code in Rust. `scripts/make-icon.swift` generates build
assets only. `scripts/build-macos.sh` packages and verifies an ad hoc signed bundle;
the icon embedded in the binary and the bundle icon must match.

Measure optimized executable and ZIP sizes before and after build changes using
the same toolchain/architecture and a clean bundle. Keep panic cleanup and existing
capabilities; report size savings separately from unmeasured performance claims.
Build outputs, local tools, test sessions, logs, and downloads stay ignored.

## Verification and handoff

Run from the repository root (the wrapper uses `.tools/` if installed):

```sh
./scripts/cargo.sh fmt --all -- --check
./scripts/cargo.sh clippy --locked --offline --all-targets -- -D warnings
./scripts/cargo.sh test --locked --offline -- --test-threads=1
./scripts/build-macos.sh release
```

Omit `--offline` when dependencies must first be fetched. Engine tests need local
TCP access; sandbox network failures are not evidence of a product regression.
For search changes, also run the HTTP fixtures documented in
`postman/simulations/discovery.context.md`, including provider failures and malformed input. Add focused tests for behavior changed or
new risks; avoid tests that merely repeat implementation. For UI changes inspect
the relevant views in an isolated `TORRENT_DATA_DIR`, including small windows and
both themes when relevant. Do not interrupt an app session the user is using.

Review the diff for unrelated changes and secrets, document measured results and
limitations, and commit on the feature branch. Push that branch when authorized;
never force-push, merge to `main`, publish a release, or modify user data as an
incidental cleanup step. User instructions take precedence over this file.
