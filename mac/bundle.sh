#!/bin/bash
# Builds a universal (Apple Silicon + Intel) "Bayer Dither.plugin" bundle and ad-hoc signs it.
set -euo pipefail
NAME="Bayer Dither"
BIN=bayer_dither
OUT="dist/$NAME.plugin"

rustup target add aarch64-apple-darwin x86_64-apple-darwin
cargo build --release --target aarch64-apple-darwin
cargo build --release --target x86_64-apple-darwin

rm -rf dist && mkdir -p "$OUT/Contents/Resources" "$OUT/Contents/MacOS"
# pipl writes these next to the build output; the exact folder depends on cargo's layout, so look them up.
find_one() { find target -name "$1" -not -path "*/deps/*" | head -1; }
RSRC=$(find_one "$BIN.rsrc"); PKGINFO=$(find_one "${BIN}_PkgInfo"); PLIST=$(find_one "${BIN}_Info.plist")
echo "rsrc=$RSRC pkginfo=$PKGINFO plist=$PLIST"
[ -n "$RSRC" ] && [ -n "$PKGINFO" ] && [ -n "$PLIST" ] || { echo "PiPL outputs not found"; find target -maxdepth 4 -name "*.rsrc" -o -maxdepth 4 -name "*_PkgInfo"; exit 1; }
cp "$RSRC"    "$OUT/Contents/Resources/$NAME.rsrc"
cp "$PKGINFO" "$OUT/Contents/PkgInfo"
cp "$PLIST"   "$OUT/Contents/Info.plist"
lipo target/{aarch64,x86_64}-apple-darwin/release/lib$BIN.dylib -create -output "$OUT/Contents/MacOS/$NAME"
/usr/libexec/PlistBuddy -c 'Set :CFBundleIdentifier "studio.plumcat.bayerdither"' "$OUT/Contents/Info.plist"

# AE 25.2+ refuses unsigned plug-ins on macOS; an ad-hoc signature is enough for local use.
codesign --force --options runtime --timestamp=none --sign - "$OUT"
codesign --verify --verbose "$OUT"
lipo -info "$OUT/Contents/MacOS/$NAME"
