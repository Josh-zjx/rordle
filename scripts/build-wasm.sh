#!/usr/bin/env bash
# Build the Rordle WASM bundle and place web-ready artifacts in web/pkg/.
#
# Usage:
#   scripts/build-wasm.sh            # release build (default)
#   scripts/build-wasm.sh --debug    # debug build (faster compile, large output)
#
# Requirements:
#   rustup target add wasm32-unknown-unknown
#   cargo install wasm-bindgen-cli
#
# After building, serve web/ with any static HTTP server, e.g.:
#   python3 -m http.server --directory web 8080
# and open http://localhost:8080/

set -euo pipefail

cd "$(dirname "$0")/.."

PROFILE="release"
PROFILE_DIR="release"
CARGO_PROFILE_FLAG="--release"

if [[ "${1:-}" == "--debug" ]]; then
    PROFILE="debug"
    PROFILE_DIR="debug"
    CARGO_PROFILE_FLAG=""
fi

WASM_BINDGEN_BIN="${WASM_BINDGEN_BIN:-$HOME/.cargo/bin/wasm-bindgen}"

if ! command -v "$WASM_BINDGEN_BIN" >/dev/null 2>&1; then
    echo "error: wasm-bindgen not found at $WASM_BINDGEN_BIN" >&2
    echo "       install with: cargo install wasm-bindgen-cli" >&2
    exit 1
fi

echo ">>> cargo build --locked --lib --target wasm32-unknown-unknown ${CARGO_PROFILE_FLAG}"
cargo build --locked --lib --target wasm32-unknown-unknown ${CARGO_PROFILE_FLAG}

WASM_INPUT="target/wasm32-unknown-unknown/${PROFILE_DIR}/rordle.wasm"
OUTPUT_DIR="web/pkg"

mkdir -p "$OUTPUT_DIR"

echo ">>> wasm-bindgen --target web --out-dir ${OUTPUT_DIR}"
"$WASM_BINDGEN_BIN" \
    --target web \
    --out-dir "$OUTPUT_DIR" \
    --no-typescript \
    "$WASM_INPUT"

echo ""
echo "Built ${PROFILE} WASM bundle in ${OUTPUT_DIR}/"
ls -lh "$OUTPUT_DIR" | sed 's/^/    /'
echo ""
echo "To serve: python3 -m http.server --directory web 8080"
