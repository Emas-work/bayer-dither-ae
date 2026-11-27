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
T=target/aarch64-apple-darwin/release
cp "$T/$BIN.rsrc"       "$OUT/Contents/Resources/$NAME.rsrc"
cp "$T/${BIN}_PkgInfo"  "$OUT/Contents/PkgInfo"
cp "$T/${BIN}_Info.plist" "$OUT/Contents/Info.plist"
lipo target/{aarch64,x86_64}-apple-darwin/release/lib$BIN.dylib -create -output "$OUT/Contents/MacOS/$NAME"
/usr/libexec/PlistBuddy -c 'Set :CFBundleIdentifier "studio.plumcat.bayerdither"' "$OUT/Contents/Info.plist"

# AE 25.2+ refuses unsigned plug-ins on macOS; an ad-hoc signature is enough for local use.
codesign --force --options runtime --timestamp=none --sign - "$OUT"
codesign --verify --verbose "$OUT"
lipo -info "$OUT/Contents/MacOS/$NAME"
