#!/usr/bin/env bash
# Zig validation battery:
#   1. native contract resolver vs oracle conformance vectors
#   2. C ABI bindings vs live theme, applied through zgui (Dear ImGui)
# Requires zig >= 0.16 (builds carry the LLVM/LLD workaround for the
# glibc 2.43+ crt1 .sframe bug in ziglang/zig#31272; fixed in zig 0.17).
set -euo pipefail
cd "$(dirname "$0")/.."

command -v zig >/dev/null || { echo "zig not found; skipping" >&2; exit 0; }
ZIGFLAGS_NOTE=""

echo "== zig-native resolver vs conformance vectors =="
mkdir -p build
zig build-exe examples/zig-conformance/omarchy_native.zig \
  -lc -fllvm -flld -OReleaseFast -femit-bin=build/zig-native
./build/zig-native --conformance conformance/vectors | tail -1

echo "== zgui (Dear ImGui) ABI verification =="
cargo build --release -p omarchy-theme-abi
(cd examples/zgui-verify && zig build && ./zig-out/bin/zgui-verify)
