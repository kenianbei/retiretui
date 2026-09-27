#!/bin/sh
# Builds the browser page into dist/, ready to serve:
#   python3 -m http.server -d crates/retiretui_web/dist
set -eu
cd "$(dirname "$0")"
rm -rf dist
# Small over fast, for this build alone: the native release keeps its own.
export CARGO_PROFILE_RELEASE_OPT_LEVEL=z
export CARGO_PROFILE_RELEASE_LTO=true
export CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1
export CARGO_PROFILE_RELEASE_STRIP=true
wasm-pack build --target web --release --no-typescript --no-pack \
    --out-dir dist/pkg --out-name retiretui
cp -R www/. dist/
