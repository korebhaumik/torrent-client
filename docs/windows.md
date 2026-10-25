# Windows builds and release checks

The Windows distribution is a portable x64 ZIP for Windows 10/11, built natively
with `x86_64-pc-windows-msvc`. The existing macOS Apple Silicon build remains a
separate artifact from the same Rust sources. Windows ARM64 and an installer are
not part of this build.

## Build and distribute

Install stable Rust and Visual Studio C++ Build Tools with the Windows SDK.
From the repository root in PowerShell:

```powershell
rustup target add x86_64-pc-windows-msvc
cargo fmt --all -- --check
cargo clippy --locked --target x86_64-pc-windows-msvc --all-targets -- -D warnings
cargo test --locked --target x86_64-pc-windows-msvc -- --test-threads=1
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\build-windows.ps1 release
```

The script recreates only its versioned staging folder under `dist/`, checks the
EXE's version resources, and writes a ZIP and SHA-256 checksum. Its version comes
from Cargo metadata; `-Profile debug` creates a separately named development ZIP.
The x64 MSVC target links the C runtime statically, so users do not need a
separate Visual C++ runtime installation. Windows builds need a working graphics
driver for the existing OpenGL renderer.

`native/build.rs` assembles an ICO from the checked-in PNG icon sizes also used
by the macOS bundle. `winresource` embeds that icon, Cargo version metadata, and
a manifest requesting normal user privileges and long-path support. Long paths
are still subject to Windows policy and filesystem limitations. Release EXEs use
the Windows GUI subsystem; `--search` attaches to the parent console when
available and supports redirected output without creating a window.

The `Desktop builds` GitHub Actions workflow runs formatting, Clippy, and the
full test suite on Windows and macOS. It also builds both release packages and
tests diagnostic CLI error reporting from the extracted Windows ZIP without
making a search request. Artifact names are `Torrent-windows-x64` and
`Torrent-macos-arm64`; each contains a ZIP and checksum. The workflow has read-only
repository permissions and does not create or publish GitHub Releases.

The Windows EXE is unsigned. If signed distribution is required, Authenticode
sign the final EXE using the release owner's certificate before ZIP creation,
then regenerate the ZIP/checksum and repeat the packaged smoke test. No signing
keys are stored in the repository.

## Windows smoke checks before release

Run these against the extracted ZIP on a Windows machine with disposable test
files. The checks are pending until completed on Windows; passing macOS tests
does not establish Windows runtime behavior.

Use a separate session for UI verification:

```powershell
$env:TORRENT_DATA_DIR = Join-Path $env:TEMP ('Torrent-QA-' + [guid]::NewGuid())
.\Torrent.exe
```

Set the download location to a temporary folder in Preferences before adding a
torrent. `TORRENT_DATA_DIR` isolates settings, metadata, the session lock, and
framework window state; downloads still use the configured download location.

- Launch from Explorer without a console window or administrator prompt; verify
  icon/version properties and operation without a separate VC++ runtime install.
- Check dark/light themes, 820×580 minimum window size, Preferences scrolling,
  display scaling, keyboard navigation, accessible labels, and disabled controls.
- Check Ctrl+K/O/F/comma/Enter, Escape, clipboard paste/copy, drag and drop,
  native file/folder picking, file opening, and Show in Explorer. Include paths
  with spaces, Unicode, and commas.
- Download from a controlled local peer; pause, close, restart, and resume.
  Check multi-file parent folders and per-torrent download destinations.
- Remove one torrent while keeping files, then remove another with file deletion.
  Verify unrelated files remain. Import a disposable legacy `.state.json` session.
- Change settings twice and restart to verify replacement of existing JSON.
  Verify a second app cannot acquire the same session lock.
- Verify CLI output with the redirection example in the ZIP's `README.txt`.
  Review any Windows firewall prompt and peer connectivity on the intended network.

Automated tests use temporary directories and local TCP peers. Added regressions
cover replacing settings/metadata in Unicode paths, preserving destinations and
removing temporary files after failed replacement, macOS/Windows shortcut labels
and input, and Windows destinations locked by another process. Search HTTP
orchestration and provider contracts are unchanged.

## Local verification

Implementation was checked on macOS Apple Silicon with Rust 1.99.1. Formatting,
Clippy with warnings denied, all 14 applicable tests, release packaging, and
strict ad hoc signature verification passed. The workflow passes actionlint and
the packaging script parses with PowerShell 7.6.8. Isolated macOS UI checks covered
Library, Add, Preferences, both themes, the minimum window size, scrolling, and
shortcuts. The Windows-only locked-file test, native compilation, package smoke
test, and interactive checks still require the first Windows workflow/manual run.

Size measurements use the same Rust toolchain/architecture and clean macOS
bundles with identical files and ad hoc signing, compressed with
`ditto -c -k --sequesterRsrc --keepParent`. The baseline is commit `636bcc7`.

| Artifact | Before (bytes) | After (bytes) | Change |
| --- | ---: | ---: | ---: |
| Optimized executable before bundle signing | 12,787,568 | 12,787,936 | +368 |
| Signed clean macOS application ZIP | 5,434,575 | 5,436,908 | +2,333 |

These are size measurements, not performance measurements. Windows executable
and ZIP sizes are printed by its packaging script when it runs on Windows; they
have not been measured on this macOS host.
