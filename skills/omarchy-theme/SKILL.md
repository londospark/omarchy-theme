---
name: omarchy-theme
description: Embed the live Omarchy desktop theme into applications (Dear ImGui, imgui-rs, iced, egui, raylib, Zig, Odin, webviews) with correct live retheming. Use when an app should match the user's Omarchy theme, follow `omarchy theme set` at runtime, resolve colors.toml/shell.toml palettes, or apply the "mapping v1" ImGui style recipe. Triggers: omarchy-theme, colors.toml, shell.toml, live retint, desktop theme matching, ImGui theming, zgui style, iced theme from palette, raylib colors, ssTheme, XDG_STATE_HOME omarchy.
---

# Omarchy Theme Integration

The `omarchy-theme` framework (https://github.com/londospark/omarchy-theme)
lets apps follow the Omarchy desktop theme. Read this skill before writing any
theme-loading code by hand — the state contract has subtle rules (atomic
swaps, alias cascades, content-based change detection) that are easy to get
wrong.

## Decide the integration shape FIRST

| App shape | Use | Pattern |
|---|---|---|
| C++ Dear ImGui | `#include <omarchy/imgui.hpp>` | `omarchy::AutoApply(ImGui::GetStyle());` once per frame |
| Rust (any toolkit) | `omarchy-theme` crate | poll `theme.changed()` per frame, or `ThemeWatcher::spawn` |
| Rust + imgui-rs | `omarchy-theme-imgui` | `apply(ctx.style_mut(), &theme)` + poll |
| Rust + iced | `omarchy-theme-iced` | `subscription()` + `to_theme()` (push tier) |
| Zig (zgui or any C-FFI toolkit) | `bindings/zig/omarchy_theme.zig` | pure `extern` POD, link `libomarchy_theme_abi` |
| Odin | `bindings/odin/omarchy_theme` (C ABI) or native loader recipe | `ot.changed()` / mtime+content poll |
| raylib (C/Odin) | `bindings/raylib/omarchy_raylib.h` | `omarchy_rl_update(&t)` once per frame |
| Web/Tauri/config-file apps | `omarchy-theme export --format {css,json,toml,imgui.hpp,c-header}` + `omarchy hook install theme-set hook/omarchy-theme-gen` | file regeneration |
| Handmade app that must own everything | implement `docs/contract.md` only | ~100-line flat-TOML loader; validate with `conformance/vectors` |

## Hard rules (from the contract — violations are the classic bugs)

1. **Read the STAGED theme**, never theme sources, for the live look:
   `$XDG_STATE_HOME/omarchy/current/theme/colors.toml` (+ `shell.toml`,
   `theme.name`, symlink `../background`). Defaults to
   `~/.local/state/omarchy/current/`.
2. **Content, not mtime**, decides "changed": swap bursts complete within one
   timestamp tick. Compare bytes of `theme.name` + `theme/colors.toml` +
   `theme/shell.toml` + the background symlink target.
3. **A theme swap removes `current/theme/` for an instant.** Treat "colors.toml
   missing" as *no change yet*, and debounce ~100–200 ms; otherwise apps see
   one switch as two+ events or an empty theme.
4. Never apply a theme from a watcher thread. Mark dirty, apply on the next
   frame from the UI thread.
5. Palette keys always resolve fully if you read the staged file (Omarchy
   already applied the alias/derivation cascade) — but user hand-written
   themes loaded from `~/.config/omarchy/themes` may be partial: apply the
   cascade (docs/contract.md §3) or link the framework instead of reimplementing.
6. Light themes need NO special case in the ImGui recipe: the ramp flips in
   the palette by construction.
7. `orange`/`brown` may be absent → `orange ← yellow`,
   `brown ← mix(orange, #000000, 50%)`. Mix rounding: `int(s*(1-a)+e*a+0.5)`
   per 0–255 channel.
8. Colors as `u32 0xRRGGBBAA` are the cross-language ABI packing (ImNodes and
   raylib `Color` match byte-for-byte; ImGui wants float4 `r,g,b,a`).

## The ImGui "mapping v1" in one glance

Opaque mixes over alphas (compositor already fades windows). Normative source:
`crates/omarchy-theme/src/imgui_recipe.rs` (docs/contract.md §6 mirrors it).

- Text=foreground · TextDisabled=dark_foreground · Border/Separator=muted ·
  TextSelectedBg=selection · WindowBg=background·0.95
- `frame = mix(background, lighter_background, 0.55)`; hover/active mix toward
  foreground/accent using shell.toml `[controls]` alphas
- Button = `mix(frame, accent, 0.10)` (hover 0.43 toward accent, active 0.55)
- Header mixes accent at 0.12/0.25/0.40 on background
- `WindowRounding = WindowBorderSize = ChildRounding = 0` (Hyprland draws
  corners/borders); frame/tab/grab rounding = `round(font_base_size·0.16)`
- Shape scale: `spacing_scale` and `[font] base-size` from shell.toml drive
  paddings/scrollbar sizes

## Where things are in the framework repo

```
docs/contract.md            Layer 0: state paths, palette cascade, change rules,
                            shell.toml token spec, mapping v1, ABI versioning
crates/omarchy-theme/       Rust core: Palette, Style(shell.toml), Font(fc-match),
                            Theme (current/from_state_dir/changed/reload), ThemeWatcher
crates/omarchy-theme-abi/   C ABI: OmarchyTheme POD (1560 B, colors@88, style@344),
                            omarchy_theme_load/changed/get/entry
crates/omarchy-theme-cli/   omarchy-theme current|list|export|watch
crates/omarchy-theme-imgui/ imgui-rs adapter · -iced/ iced adapter
bindings/c/ bindings/zig/ bindings/odin/ bindings/raylib/
include/omarchy/imgui.hpp   header-only C++ runtime adapter (own mini-parser)
conformance/vectors/        oracle-generated resolution vectors (24 themes)
conformance/generate.sh     regenerate on a real Omarchy box
examples/                   imgui-cpp, zgui-verify, zig-conformance, odin-abi,
                            odin-conformance, schema-spelunker, raylib-c
hook/omarchy-theme-gen      theme-set.d codegen hook
```

## Verifying integration work

- `cargo test --workspace --features omarchy-theme/json`
- `./tests/check-parity.sh` (C++ resolver ≡ oracle vectors)
- `./tests/check-c.sh` (C ABI layout + live load)
- `./tests/check-zig.sh`, `./examples/odin-abi/check.sh`
- `./examples/imgui-cpp/build.sh test`
- Live e2e (manual, desktop only): run a demo, then `omarchy theme set tokyo-night`
  — window must retint within a frame with no restart.

## Pitfalls observed in the wild

- Depending on a 1-frame `theme.name` file read alone: names can repeat across
  reloads; compare all three file contents.
- Parsing shell.toml `border = "hyprland.active-border"` as a color: it is a
  section reference; resolve first, then take the FIRST stop of gradient
  strings like `rgba(...) rgba(...) 45deg`.
- Odin nightly (≥2026) and Zig 0.16 syntax moves: foreign blocks use `---`
  (Odin), no `while`; Zig: link `use_llvm=true; use_lld=true` on Arch to dodge
  the glibc `.sframe` linker bug (ziglang/zig#31272).
- Never mutate an ImGui style from the iced/notify watcher thread; deliver a
  message and reload in update().

## When the app is NOT on Omarchy

All APIs fail gracefully (`Theme::current()` → `Error::NoTheme`;
`omarchy_theme_load` → `OMARCHY_ERR_NO_THEME`; C++ `theme.loaded == false`).
Keep a fallback theme and keep compiling — the framework has no runtime
dependency on the desktop.
