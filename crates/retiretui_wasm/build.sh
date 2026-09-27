#!/bin/sh
# Builds the bindings for a browser into pkg/, typed by bindings/.
set -eu
cd "$(dirname "$0")"
# Small over fast, for this build alone: the native release keeps its own.
export CARGO_PROFILE_RELEASE_OPT_LEVEL=z
export CARGO_PROFILE_RELEASE_LTO=true
export CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1
export CARGO_PROFILE_RELEASE_STRIP=true
# Weak references free what the page drops, as a garbage-collected value is.
wasm-pack build --target web --release --weak-refs --no-pack --out-dir pkg
