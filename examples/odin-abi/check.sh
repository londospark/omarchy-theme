#!/usr/bin/env bash
# Build and run the Odin ABI example, then cross-check its output against
# the Rust CLI export for the same live theme. Validates: header <-> Rust
# struct layout at runtime, symbol naming, the Odin binding package, and
# color packing byte order across the FFI boundary.
set -euo pipefail
cd "$(dirname "$0")/../.."

ROOT=$PWD
command -v odin >/dev/null || { echo "odin not on PATH; skipping" >&2; exit 0; }

cargo build --release -p omarchy-theme-abi
cargo build --release -p omarchy-theme-cli

export LIBRARY_PATH="$ROOT/target/release${LIBRARY_PATH:+:$LIBRARY_PATH}"
export LD_LIBRARY_PATH="$ROOT/target/release${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"

odin build examples/odin-abi -collection:omarchy=bindings/odin \
  -o:none -out:build/odin-abi

./build/odin-abi > build/odin-abi.tsv
cat build/odin-abi.tsv

# Cross-checks against the Rust-side ground truth (live theme).
fail=0
check() {
  local label="$1" want="$2" got="$3"
  if [[ $got != $want ]]; then
    echo "MISMATCH $label: odin=$got rust=$want" >&2; fail=1
  fi
}

theme=$(awk -F'\t' '/^theme/{gsub(/ /,"",$2);print $2}' build/odin-abi.tsv)
odin_accent=$(awk -F'\t' '/^accent/{gsub(/ /,"",$2);print tolower($2)}' build/odin-abi.tsv)
odin_bg=$(awk -F'\t' '/^background/{gsub(/ /,"",$2);print tolower($2)}' build/odin-abi.tsv)
odin_cursor=$(awk -F'\t' '/^get_cursor/{print tolower($2)}' build/odin-abi.tsv)
odin_font=$(awk -F'\t' '/^font/{print $2}' build/odin-abi.tsv)

rust_accent=$(./target/release/omarchy-theme export --format json |
  python3 -c 'import json,sys; p=json.load(sys.stdin)["palette"]; print(p["accent"].lower())')
rust_cursor=$(./target/release/omarchy-theme export --format json |
  python3 -c 'import json,sys; p=json.load(sys.stdin)["palette"]; print(p["cursor"].lower())')
rust_bg=$(./target/release/omarchy-theme export --format json |
  python3 -c 'import json,sys; p=json.load(sys.stdin)["palette"]; print(p["background"].lower())')
rust_name=$(./target/release/omarchy-theme current | cut -f1)
rust_font=$(./target/release/omarchy-theme export --format json |
  python3 -c 'import json,sys; print(json.load(sys.stdin)["font"]["family"])')

check theme "$rust_name" "$theme"
check accent "$rust_accent" "$odin_accent"
check background "$rust_bg" "$odin_bg"
check cursor "$rust_cursor" "$odin_cursor"
check font "$rust_font" "$odin_font"

(( fail == 0 )) && echo "odin-abi: all cross-checks passed"
exit $fail
