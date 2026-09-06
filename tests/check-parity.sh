#!/usr/bin/env bash
# Cross-language palette parity: every conformance vector (resolved by the
# omarchy-theme-color oracle) must be reproduced, key-for-key, by the
# header-only C++ implementation in include/omarchy/imgui.hpp.
#
#   ./tests/check-parity.sh
set -euo pipefail
cd "$(dirname "$0")/.."

[[ -f build/imgui-vendor/imgui.h ]] || { echo "missing build/imgui-vendor/imgui.h (see examples/README)" >&2; exit 1; }
mkdir -p build
g++ -std=c++17 -Iinclude -Ibuild/imgui-vendor -o build/cpp-parity tests/cpp-parity.cpp

fail=0
for vec in conformance/vectors/*.json; do
  slug=$(python3 -c "import json,sys;print(json.load(open('$vec'))['theme'])")
  slug=$(basename "$vec" .json)
  src="conformance/vectors/$slug.colors.toml"
  [[ -f $src ]] || src=$(python3 -c "import json,sys;print(json.load(open('$vec'))['colors_file'])")
  marker=$(python3 -c "import json,sys;print('light' if json.load(open('$vec'))['light_mode_marker'] else '')")
  [[ -f $src ]] || { echo "skip $slug (no colors source)"; continue; }
  python3 -c "
import json
v = json.load(open('$vec'))['resolved']
for k in sorted(v): print(f'{k}\t{v[k]}')" | LC_ALL=C sort > build/expected.tsv

  "./build/cpp-parity" "$src" $marker > build/got.tsv

  if ! diff -q build/expected.tsv build/got.tsv >/dev/null; then
    echo "FAIL $slug"
    diff build/expected.tsv build/got.tsv | head -10
    fail=1
  else
    echo "ok   $slug"
  fi
done
exit $fail
