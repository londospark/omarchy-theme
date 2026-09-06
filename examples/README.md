# Examples & how they're checked

| Example | Language | Proof |
|---|---|---|
| `imgui-cpp/` | C++ | `build.sh test` links real Dear ImGui (docking), applies the runtime header, asserts style colors == live palette; `build.sh run` opens the live demo (needs `glfw` + GL headers) |
| `odin-abi/` | Odin | builds against the actual `libomarchy_theme_abi.so`; `check.sh` diffs every value against `omarchy-theme export --format json` |
| `odin-conformance/` | Odin | pure-contract resolver verified against all conformance vectors (the oracle test for a from-scratch loader) |
| `schema-spelunker/` | Odin | PR-ready native bridge for londospark/Schema-Spelunker (see `docs/recipes/schema-spelunker.md`) |
| `raylib-c/` | C | compile-checked with vendored raylib headers (`cc -fsyntax-only`); linkable with a system raylib |
| `zgui-verify/` | Zig | initializes a real **zgui** (Dear ImGui) context, applies "mapping v1" from the C ABI bindings, asserts applied colors == live palette |
| `zig-conformance/` | Zig | pure-contract resolver (like odin-conformance), checked against all vectors via `zig run`/`build-exe` |

Shared headers live in `build/` after vendoring (git-ignored):

```sh
mkdir -p build/imgui-vendor build/raylib-vendor
for f in imgui.h imconfig.h; do
  curl -sL -o build/imgui-vendor/$f https://raw.githubusercontent.com/ocornut/imgui/docking/$f
done
for f in imgui.cpp imgui_draw.cpp imgui_tables.cpp imgui_widgets.cpp imgui_internal.h imstb_rectpack.h imstb_textedit.h imstb_truetype.h; do
  curl -sL -o build/imgui-vendor/$f https://raw.githubusercontent.com/ocornut/imgui/docking/$f
done
curl -sL -o build/raylib-vendor/raylib.h https://raw.githubusercontent.com/raysan5/raylib/5.5/src/raylib.h
```

Run the full cross-language battery:

```sh
cargo test --workspace --features omarchy-theme/json
./tests/check-parity.sh         # C++ palette ≡ oracle vectors
./tests/check-c.sh              # C ABI runtime + layout asserts
./examples/odin-abi/check.sh    # Odin bindings ≡ Rust CLI
./examples/imgui-cpp/build.sh test
./tests/check-zig.sh              # zig >= 0.16; zgui fetched via zig pkg manager
```
