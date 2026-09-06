# Vision

`omarchy-theme` is the missing layer between the [Omarchy](https://omarchy.org)
desktop and **applications built to run on it**. Omarchy themes everything the
desktop controls — terminal, editor, bar, browser — from one palette
(`colors.toml`) plus one style sheet (`shell.toml`). But the moment you build
your own app (Dear ImGui, egui, iced, raylib, a webview), the desktop's theme
stops at the window border. This framework makes "it matches the desktop, and
retints live when I theme-switch" a two-line feature in any toolkit, in any
language.

## Principles

1. **Contract first, code second.** Omarchy's staged state under
   `$XDG_STATE_HOME/omarchy/current/` *is* the API. We document it
   (`docs/contract.md`), publish conformance vectors, and treat implementations
   as swappable. A ~100-line loader in your own language is a valid client —
   the Schema-Spelunker example proves it in Odin with zero dependencies.
2. **Toolkit-neutral, adapter-shaped.** One resolution (palette + style tokens
   + font) feeds many toolkits. Recipes and adapters convert; they never
   re-resolve semantics, so the desktop's meaning and your widget's meaning
   can't drift.
3. **Immediate-mode first, retained-mode well.** For loop-paint apps
   (ImGui, raylib, egui) a once-per-frame `changed()` poll is enough; for
   event-driven apps (iced, Qt, GTK) a debounced push subscription. Nobody
   injects mutation into your UI thread behind your back — auto-apply helpers
   apply *on the next frame*, from the frame loop.
4. **Polyglot by C ABI, not by binding generator.** A frozen, POD-shaped C
   interface (versioned, slot-indexed, no allocation across the boundary)
   makes every language with a C FFI — Odin, Zig, C3, Go, C#, Python — a
   first-class consumer. We hand-write bindings for the languages we dogfood.
5. **Zero-dependency escape hatch.** The export CLI turns any theme into
   `json | css | toml | c-header | imgui.hpp` files; a `theme-set.d` hook
   regenerates them atomically. Apps that hate libraries can be pure
   codegen consumers. (App-specific formats — e.g. Schema-Spelunker's
   `.ssTheme` — live as recipes/examples, not core formats.)

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│ Toolkits & apps                                             │
│  imgui.hpp (C++) · imgui-rs/egui (Rust, later) · iced crate │
│  omarchy_raylib.h · Odin bindings · CSS (Tauri/web) · codegen│
├────────────────────────────────────────────────────────────┤
│ omarchy-theme-abi (C ABI: POD struct, poll+get, frozen v1)  │
│ omarchy-theme-cli  (export formats, watch -- cmd)           │
├────────────────────────────────────────────────────────────┤
│ omarchy-theme (Rust core): Palette · Style · Font · Theme   │
│   + ThemeWatcher (inotify, debounced, swap-tolerant)         │
├────────────────────────────────────────────────────────────┤
│ THE CONTRACT (docs/contract.md)                             │
│  $XDG_STATE_HOME/omarchy/current/theme{,.name} · background │
│  colors.toml cascade · shell.toml tokens · mix rules        │
│  conformance/ vectors ← oracle: omarchy-theme-color         │
└────────────────────────────────────────────────────────────┘
```

## Adapter matrix (current + planned)

| Target | Form | Status |
|---|---|---|
| Dear ImGui (C++) | `include/omarchy/imgui.hpp` runtime header + `imgui.hpp` codegen | ✅ v1, smoke-tested |
| Odin (any FFI app) | `bindings/odin/omarchy_theme` (C ABI) + native contract loader | ✅ v1, ABI + conformance tested |
| raylib | `bindings/raylib/omarchy_raylib.h` (POD struct + Color conversion) | ✅ v1, example-checked |
| iced | `omarchy-theme-iced` crate (`to_theme` + `subscription`) | ✅ v1, example-checked |
| C / anything with a C ABI | `bindings/c/omarchy_theme.h` | ✅ v1, smoke-tested |
| Web / Tauri / Electron | `--format css` + theme-set hook | ✅ v1 |
| Config-file apps | `--format toml/json/c-header`, `watch --`, hook | ✅ v1 |
| egui | `omarchy-theme-egui` crate (Visuals mapping) | planned v0.2 |
| Qt (QSS palette), Slint, GTK4 | export + adapters | planned |
| Odin imgui-rs-style wrappers | examples | planned |

## Relationship to existing crates

`omarchy-themes` (crates.io, pixbroker, 2026-09) solves palette resolution +
watching in Rust and inspired parts of our docs; it is palette-only (no
`shell.toml` tokens, no font, no toolkit adapters, no C ABI). We implement
the cascade ourselves rather than depend on it, and pin correctness with
conformance vectors generated from the real `omarchy-theme-color` script. If
both projects survive, a bridge feature (`From<omarchy_themes::Palette>`) is
a 30-line courtesy.

## Non-goals

* Replacing Omarchy's `*.tpl` system — if your app reads a config file, a
  template is *better* than this framework.
* Theme authoring/editing (see Aether, omarchy-studio).
* Repainting anything outside your own window; the desktop already owns that.
* Chasing per-theme hand-picked widget styles: one documented recipe
  ("mapping v1") applied to every theme, tuned once, reviewable by Omarchy
  maintainers.

## North star

`~/dev/schema_spelunker` and its sibling handmade apps open with the user's
Omarchy theme on frame one and retint the instant the theme changes — with
less integration code than "dark mode" took to get wrong everywhere else.
