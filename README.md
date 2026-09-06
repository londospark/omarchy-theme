# omarchy-theme

[![CI](https://github.com/londospark/omarchy-theme/actions/workflows/ci.yml/badge.svg)](https://github.com/londospark/omarchy-theme/actions/workflows/ci.yml)
[![license: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)


Follow the Omarchy desktop theme from **any** app or toolkit: resolved
palette (the exact cascade the desktop itself uses), shell style tokens
(control-state alphas, border gradients, spacing, font scale), the UI font,
the live background — and a correct notification when the user theme-switches,
so your windows retint without restart.

```
cargo build --release                 # core, CLI, C ABI lib
./conformance/generate.sh             # oracle vectors for this machine
cargo test --workspace --features omarchy-theme/json
./tests/check-parity.sh               # C++ header ≡ oracle ≡ vectors
./tests/check-c.sh                    # C ABI + header smoke test
./examples/imgui-cpp/build.sh test    # real ImGui, runtime assertions
./examples/odin-abi/check.sh          # Odin bindings ≡ Rust CLI
./tests/check-zig.sh                  # Zig: native resolver ≡ vectors + zgui applies live
```

## Agent skill

This repo ships a [skill](skills/omarchy-theme/SKILL.md) (`skills/omarchy-theme/`)
teaching coding agents (opencode, Claude Code, etc.) how to integrate the
framework correctly — drop it in `.opencode/skills/`, `~/.agents/skills/`, or
`~/.claude/skills/` of any project that should follow the desktop theme.

## Choose your integration

| You are writing | Use | Live retint |
|---|---|---|
| C++ Dear ImGui app | [`include/omarchy/imgui.hpp`](include/omarchy/imgui.hpp) | `AutoApply(style)` per frame |
| Odin app (any FFI GUI) | [`bindings/odin/omarchy_theme`](bindings/odin/omarchy_theme) or the native recipe in [`examples/schema-spelunker`](examples/schema-spelunker) | poll `changed()` / signature |
| Rust Dear ImGui (imgui-rs) | [`crates/omarchy-theme-imgui`](crates/omarchy-theme-imgui) | `theme.changed()` per frame |
| Rust egui | `omarchy-theme` core + adapter (planned v0.2) | — |
| Rust iced | [`crates/omarchy-theme-iced`](crates/omarchy-theme-iced) | `subscription()` |
| raylib (C or Odin) | [`bindings/raylib/omarchy_raylib.h`](bindings/raylib/omarchy_raylib.h) | `omarchy_rl_update(&t)` per frame |
| Zig (zgui / Dear ImGui validated) | [`bindings/zig/omarchy_theme.zig`](bindings/zig/omarchy_theme.zig) — pure extern POD, or native loader | `ot.changed()` per frame |
| Go / C# / anything else with C FFI | [`bindings/c/omarchy_theme.h`](bindings/c/omarchy_theme.h) → cdylib | `omarchy_theme_changed()` |
| Webview / Tauri / config-file app | `omarchy-theme export` + [`hook/omarchy-theme-gen`](hook/omarchy-theme-gen) | file regeneration |
| You want NO dependency at all | [`docs/contract.md`](docs/contract.md) — ~100-line loader, vectors prove it | your own poll |

## Quickstarts

**C++ (Dear ImGui)**

```cpp
#include <omarchy/imgui.hpp>

while (running) {
    omarchy::AutoApply(ImGui::GetStyle());   // load once, re-apply on change
    NewFrame(); … DrawFrame();
}
```

**Rust (core + imgui-rs)**

```rust
let mut theme = omarchy_theme::Theme::current()?.with_font();
let mut ctx = imgui::Context::create();
omarchy_theme_imgui::apply(ctx.style_mut(), &theme);
loop {
    if theme.changed() {
        theme.reload()?;
        omarchy_theme_imgui::apply(ctx.style_mut(), &theme);
    }
    // push-tier alternative: ThemeWatcher::spawn(|t| { tx.send(t); true })
}
```

**iced (Rust)** — see [`crates/omarchy-theme-iced/src/lib.rs`](crates/omarchy-theme-iced/src/lib.rs)

```rust
fn subscription(&self) -> Subscription<Msg> { omarchy_theme_iced::subscription() }
fn theme(&self) -> Theme { self.theme.clone() }
// Msg::Omarchy(_) => self.theme = omarchy_theme_iced::to_theme(&Theme::current()?)
```

**Odin via C ABI**

```odin
import ot "omarchy:omarchy_theme"
t, ok := ot.load()
for running {                       // your frame loop
    if ot.changed() { t, _ = ot.load() }
    accent := ot.unpack(t.colors[int(ot.Color_Slot.Accent)])   // 0xRRGGBBAA
}
```
Build: `cargo build --release -p omarchy-theme-abi`, then link
`-lomarchy_theme_abi` with `LIBRARY_PATH`/`LD_LIBRARY_PATH` pointed at
`target/release` (or install the lib).

**raylib**

```c
OmarchyTheme t = {0}; t.abi_size = sizeof t; omarchy_theme_load(&t);
while (!WindowShouldClose()) {
    omarchy_rl_update(&t);                              // 1 line, live
    omarchy_rl_clear(&t, 0);                            // background
    DrawTextEx(font, "hi", pos, 28, 1, omarchy_rl(&t, OMC_ACCENT));
}
```

**Codegen (zero runtime deps)**

```bash
omarchy-theme export --format imgui.hpp --out theme_imgui.hpp
omarchy-theme export --format css       --out theme.css
omarchy-theme export --theme tokyo-night --format json   # any theme, not just live
omarchy hook install theme-set ~/path/to/hook/omarchy-theme-gen   # keep them fresh
```

## Layout

```
docs/contract.md         the language-neutral spec (start here)
docs/VISION.md           framework thesis + adapter roadmap
conformance/             generate.sh + vectors/<theme>.json (oracle test data)
crates/
  omarchy-theme/         Rust core: Palette · Style · Font · Theme · ThemeWatcher
  omarchy-theme-imgui/   imgui-rs adapter (mapping v1)
  omarchy-theme-abi/     C ABI cdylib/staticlib (POD OmarchyTheme, poll+get)
  omarchy-theme-cli/     `omarchy-theme` binary: current|list|export|watch
  omarchy-theme-iced/    iced adapter (to_theme + subscription)
bindings/
  c/omarchy_theme.h      frozen ABI v1: size 1560, colors@88, style@344
  zig/omarchy_theme.zig  pure-Zig extern bindings (comptime layout asserts)
  odin/omarchy_theme/    Odin bindings (foreign + friendly API)
  raylib/omarchy_raylib.h  raylib conversions over the POD struct
include/omarchy/imgui.hpp  header-only Dear ImGui runtime adapter
hook/omarchy-theme-gen   theme-set.d example regenerating export files
examples/                imgui-cpp · odin-abi · odin-conformance · zgui-verify ·
                         zig-conformance · schema-spelunker (native recipe) · raylib-c
tests/                   parity + ABI + smoke harnesses
```

## Correctness

Three independent implementations of the resolution cascade (Rust core, C++
header, Odin native example) are tested key-for-key against
**conformance vectors recorded from the real `omarchy-theme-color` script**
for every installed theme (22/22 stock + user themes + staged current). The
C ABI layout is frozen by constants asserted in Rust, the C header, and the
Odin bindings; color packing (`0xRRGGBBAA`) matches ImNodes and raylib byte
order. See docs/contract.md §8 for what is allowed to change.

## Status

v0.1 — built and tested on a live Omarchy session; the desktop theme is
followed end-to-end. Public APIs: contract + C ABI are frozen; Rust/C++/iced
APIs may add fields until v0.2.0.

## License

MIT.
