#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/.."
if [[ -x .tools/cargo/bin/cargo ]]; then
  export CARGO_HOME="$PWD/.tools/cargo"
  export RUSTUP_HOME="$PWD/.tools/rustup"
  export PATH="$CARGO_HOME/bin:$PATH"
fi
profile="${1:-release}"
case "$profile" in
  debug) cargo build --locked ;;
  release) cargo build --release --locked ;;
  *) printf 'Usage: %s [debug|release]\n' "$0" >&2; exit 2 ;;
esac
bundle="$PWD/dist/Torrent.app"
mkdir -p "$bundle/Contents/MacOS" "$bundle/Contents/Resources"
cp "target/$profile/torrent-desktop" "$bundle/Contents/MacOS/Torrent"
cat > "$bundle/Contents/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleDevelopmentRegion</key><string>en</string>
<key>CFBundleExecutable</key><string>Torrent</string>
<key>CFBundleIdentifier</key><string>app.torrent.desktop</string>
<key>CFBundleName</key><string>Torrent</string>
<key>CFBundleDisplayName</key><string>Torrent</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>CFBundleShortVersionString</key><string>2.1.0</string>
<key>CFBundleVersion</key><string>5</string>
<key>CFBundleIconFile</key><string>TorrentMagnet</string>
<key>LSMinimumSystemVersion</key><string>11.0</string>
<key>LSApplicationCategoryType</key><string>public.app-category.utilities</string>
<key>NSHighResolutionCapable</key><true/>
<key>NSHumanReadableCopyright</key><string>Torrent Desktop</string>
</dict></plist>
PLIST
if [[ ! -f native/assets/Torrent.icns || scripts/make-icon.swift -nt native/assets/Torrent.icns || native/assets/brand.svg -nt native/assets/Torrent.icns ]]; then
  mkdir -p .tools/swift-cache
  swift -module-cache-path .tools/swift-cache scripts/make-icon.swift
fi
cp native/assets/Torrent.icns "$bundle/Contents/Resources/TorrentMagnet.icns"
rm -f "$bundle/Contents/Resources/Torrent.icns"
codesign --force --deep --sign - "$bundle"
codesign --verify --deep --strict "$bundle"
printf 'Built: %s\n' "$bundle"
