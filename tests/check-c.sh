#!/usr/bin/env bash
# C header + ABI smoke test against the live desktop theme.
set -euo pipefail
cd "$(dirname "$0")/.."
cargo build --release -p omarchy-theme-abi
cc -std=c11 -Wall -Wextra -Wpedantic -Ibindings/c tests/c-smoke.c \
   -Ltarget/release -lomarchy_theme_abi -o build/c-smoke
LD_LIBRARY_PATH=target/release ./build/c-smoke
