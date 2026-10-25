# Native app optimization

Measured on 2026-10-25 on Apple Silicon, relative to `main` at `24a9e45`.
The same local Rust toolchain, release packaging script, icon assets, and ZIP
command were used on both builds.

| Artifact | Before (bytes) | After (bytes) | Reduction |
| --- | ---: | ---: | ---: |
| Stripped release executable | 13,438,608 | 12,787,568 | 4.84% |
| Signed macOS application ZIP | 6,246,949 | 5,434,402 | 13.01% |

The release profile uses `opt-level = "s"` with the existing thin LTO,
single codegen unit, and stripping. Panic unwinding is retained. Tokio's broad
`full` feature was replaced with explicit runtime, macro, channel, timer, and
network requirements; its unused process, signal, standard-input, and alternative
lock features are no longer enabled. Every direct dependency has a source use.
The clipboard dependency no longer requests default features directly, although
egui still enables image support transitively. Accessibility, bundled font
fallbacks, persistence, clipboard behavior, and the torrent engine are retained.

The UI clones an immutable `Arc<Snapshot>` per frame instead of every torrent and
file. Theme configuration is rebuilt only on the first frame or after a theme
change. Search regexes compile once, query tokens are reused within provider
results, and empty library filters skip unnecessary lowercase allocations. The
five-minute search cache now prunes expired entries and holds at most 64 entries.
These changes remove repeated work and bound cache growth; they are not claims
of measured download-throughput or frame-rate improvements.

Dialogs and drawers moved verbatim to `app/dialogs.rs` and `app/details.rs`.
Search normalization and caching now have separate modules. Engine commands and
completion handling moved out of the `select!` macro into a single-owner worker,
making the command code readable by rustfmt and Clippy. Saved JSON and command
semantics are unchanged. `AGENTS.md` defines ownership, data invariants, dependency
discipline, branch rules, and verification for future changes.

Validation passed:

- Formatting and Clippy for all targets with warnings denied.
- Six unit tests: control interaction/layout, cache expiry/capacity, RSS/CDATAs,
  normalization, Unicode, and description bounds.
- Five integration tests: real local peer transfer, paused restart/recovery,
  cached metadata, multi-file paths, file retention/deletion, legacy import,
  state locking, and input validation. Retained snapshots are checked for
  consistency across a worker update.
- The production HTTP client against the local Postman simulation, with request
  logs confirming all four adapters and error/fallback routes. See
  `postman/simulations/discovery.context.md` for scope and limitations.
- Isolated app checks: library, dark/light theme switching, Add keyboard shortcut
  and validation, torrent-file tab, Preferences, details, Remove cancellation,
  and centered filter empty state. Dialog/drawer method bodies were also compared
  with `main` after relocation and were unchanged.
- Optimized release bundle and strict ad hoc signature verification.

Tests use temporary files and local peers, not the user's live session. This
coverage found no regressions; it does not establish live index availability or
benchmark sustained public-swarm throughput.
