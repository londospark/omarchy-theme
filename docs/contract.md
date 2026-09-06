# The Omarchy Theme Contract (Layer 0)

Version 1.0 · 2026-09

This document is the language-neutral specification that every implementation
in this repository (Rust core, C ABI, C++ header, Odin bindings, export CLI)
obeys, and that *third-party* implementations may rely on. An app may skip
this project's code entirely and implement only this document — the Odin
example (`examples/odin-conformance/`) is exactly such an implementation, and
the conformance vectors exist to prove you got it right.

---

## 1. Where the theme lives

The live theme is **staged**, not read from theme sources:

| Path | Contents |
|---|---|
| `$XDG_STATE_HOME/omarchy/current/theme.name` | theme slug, e.g. `catppuccin` (`$XDG_STATE_HOME` defaults to `~/.local/state`) |
| `…/current/theme/colors.toml` | the palette (§2), already merged with any user overlay and normalized |
| `…/current/theme/shell.toml` | style tokens (§5): control states, surfaces, spacing, font scale |
| `…/current/theme/<name>` | every generated file (alacritty.toml, btop.theme, …) |
| `…/current/background` | symlink to the active wallpaper |

`omarchy-theme-set` renders the chosen theme (stock copy + user overlay +
template generation) into a staging directory and **swaps it in atomically**
(rename). `current/` itself persists across the swap, so a file-watch on
`current/` survives every theme change, and consumers never observe a
half-rendered theme file — only a moment where `current/theme` is missing.

Theme sources (for rendering *non-active* themes, e.g. a preview):

* `$OMARCHY_PATH/themes/<slug>/` (packaged default: `/usr/share/omarchy/themes/`)
* `~/.config/omarchy/themes/<slug>/` — **user files win per-file** over stock
* legacy note: `~/.config/omarchy/current/theme/` is the *old* location; do
  not read it.

Slug normalization (as performed by `omarchy-theme-set`): lowercase,
whitespace → `-`.

## 2. colors.toml — the palette

Line-based flat TOML: `key = value`, one per line. Keys match
`[A-Za-z0-9_-]+`; values match `[A-Za-z0-9#(),._+/% -]*` (after quote
stripping); lines failing these filters are skipped, last definition wins.
Quoted values are taken between the first quote pair (inline comments are
thus ignored); unquoted values are trimmed.

**Canonical keys** (every staged theme has all of them; hand-written themes
should):

```
mode                       "dark" | "light"
accent selection muted
background dark_background darker_background lighter_background
foreground dark_foreground light_foreground bright_foreground
red yellow green cyan blue magenta            (+ optional orange, brown)
bright_red bright_yellow bright_green bright_cyan bright_blue bright_magenta
```

Values are `#rrggbb` hex (lowercase or not; parse case-insensitively).
Consumers should also accept `#rgb`, `#rrggbbaa`, and `rgb()/rgba()` since
user themes may use them.

**The neutral ramp**: `darker_background → dark_background → background →
selection → lighter_background → … → bright_foreground` is monotonic in
darkness for dark themes; light themes flip the direction. `selection` is a
text-selection background *stop* on that ramp; `muted` = de-emphasis color,
also ANSI color8; `accent` is the interaction hue.

## 3. Resolution cascade (normative)

Themes may define any subset; the desktop resolves the rest. This order is
the contract (implemented by `omarchy-theme-color`, ported 1:1 in
`crates/omarchy-theme/src/palette.rs` — validate against the vectors in
`conformance/`). "Present" means *set and non-empty*.

1. Legacy short-name aliases, canonical wins:
   `bg,dark_bg,darker_bg,lighter_bg,fg,dark_fg,light_fg,bright_fg` ↔
   `background…bright_foreground`.
2. `background ← color0`, `foreground ← color7`; then if defined,
   `color0 := background`, `color7 := foreground`.
3. `red,green,yellow,blue,magenta,cyan ← color1..6`,
   `bright_red…bright_cyan ← color9..14`;
   `magenta ← purple`, `bright_magenta ← bright_purple`.
4. `light_foreground ← color7 ?? foreground`;
   `bright_foreground ← color15 ?? foreground`;
   `cursor := bright_foreground` (unconditional);
   `lighter_background ← color0 ?? background`;
   `dark_foreground ← color8 ?? foreground`;
   `muted ← color8 ?? dark_foreground`;
   `selection ← selection_background ?? color8 ?? color0 ?? background`;
   `selection_background := selection` (if absent);
   `selection_foreground := bright_foreground` (if absent);
   `orange ← yellow`; `brown ← mix(orange, #000000, 50%)`.
5. Derived shades:
   `dark_background ← mix(background, #000000, 25%)`;
   `darker_background ← mix(background, #000000, 50%)`;
   `bright_<c> ← mix(<c>, #ffffff, 20%)` for red/yellow/green/cyan/blue/magenta;
   `purple := magenta`, `bright_purple := bright_magenta`.
6. ANSI back-fill `color1..15 ← semantic names`; `color8 ← muted`.
7. Back-propagate canonical → legacy short names.
8. **mode** precedence: `mode` key → `theme_type` key → `light.mode` file
   beside colors.toml → luminance auto-detect: parse `background` as exactly
   `#rrggbb`, sum channels, `> 382` ⇒ light else dark → default `dark`.
   Then `theme_type := mode`.

**mix**: per channel, `int(start*(1-a) + end*a + 0.5)` on 0–255 integers;
amount accepts `0.25`, `25`, or `25%` (values > 1 divide by 100), clamped to
[0,1]. Output lowercase `#rrggbb`.

## 4. Change detection (normative for all tiers)

The observable state = **content** of `theme.name` + `theme/colors.toml` +
`theme/shell.toml` + the `background` symlink target. Content, not mtime:
filesystem timestamp granularity (tmpfs/ext4) is coarser than a theme swap.

Rules every implementation follows:

1. A **poll** function (`changed()`) is safe at once-per-frame cost: a few
   small file reads is the entire budget.
2. A **swap burst** (directory momentarily missing, multiple inotify events)
   must surface as *exactly one* change: treat "theme directory absent" as
   *no change yet*, and debounce ~100–200 ms before confirming via the
   content signature.
3. Push watchers deliver on a background thread; applying must happen on the
   app thread (auto-apply helpers are "mark dirty, apply next frame").
4. The `theme-set.d` hook chain fires ~1s after the swap and receives the
   theme slug as `$1` — use it only for out-of-process consumers (export
   regeneration), never as the primary signal.

## 5. shell.toml — style tokens

Generated from `default/themed/shell.toml.tpl`; flat `[section]` blocks.
A theme may replace the whole file (`shell.toml`) or one section
(`shell.<section>.toml`). Key forms:

* strings in quotes (colors, gradient specs), bare numbers/booleans
* **references**: `border = "hyprland.active-border"` resolves to
  `sections["hyprland"]["active-border"]` within the same file
* **gradients**: `"rgba(0,0,0,1) rgba(0,0,0,0) 45deg"` — consumers that
  cannot render them use the **first stop** (flat color) — same convention
  as the shell's `Color.<section>.border` singletons
* border widths: CSS-style lists (`N`, `"Y X"`, `"T X B"`, `"T R B L"`) —
  non-UI consumers take the first scalar

Sections: `bar`, `hyprland` (`active-border`, `active-border-foreground`),
`controls` (state chrome: `<state>-color`, `-fill-alpha`, `-border`,
`-border-width`, `-border-alpha` for states `normal`, `hover-cursor`,
`focus`, `selected`, plus `pressed-fill-alpha`, `selection-fill-alpha`),
`spacing` (`scale`, optional pinned tokens; commented defaults ARE the
defaults: xxs=2, xs=3, sm=4, md=6, lg=8, xl=10, xxl=12, xxxl=14, huge=18,
control-gap=8, control-padding-x=10, control-padding-y=6, input-padding-y=7,
control-height=28, popup-row-height=28, row-gap=8, row-padding-x=12,
label-gap=4, panel-gap=14, panel-padding=18, popup-padding=14,
dropdown-width=240, searchable-dropdown-width=260, number-field-width=120,
searchable-popup-min-height=220), `font` (`base-size = 12`, type-scale
tokens: caption 10, body-small 11, body 12, subtitle 13, title 14,
heading 16, display 24, display-large 28, icon-small 11, icon 14,
icon-large 18), and surfaces `popups`, `tooltip`, `notifications`,
`launcher`, `menu`, `polkit`, `lock`, `image-picker`
(`background[-alpha]`, `text`, `border[-alpha]`, `scrim[-alpha]`).

Note: **no corner-radius token ships today** — adapters derive shape from
`font.base-size` (see §6 rounding rules); themes wanting different rounding
override via their own `shell.toml`.

## 6. Dear ImGui recipe ("mapping v1")

Normative Rust implementation: `crates/omarchy-theme/src/imgui_recipe.rs`;
mirrored by the runtime C++ header and the `--format imgui.hpp` export.

Principles:

1. *Opaque mixes over alphas* (compositor already applies window alpha).
   `X.Y` = `mix(X, Y, α)`; α values from §5 controls when present.
2. Accent drives interaction; the background ramp + foreground drive chrome.
3. Light themes need no special case: the ramp direction is flipped in the
   palette by construction.

Bake-offs: `frame = mix(background, lighter_background, 0.55)`,
`frame_hover = mix(frame, foreground, max(hover-fill, 0.08))`,
`frame_active = mix(frame, accent, max(pressed-fill, 0.15))`,
`button = mix(frame, accent, 0.10)` (hover/active at 0.43/0.55 toward
accent), headers at accent 0.12/0.25/0.40 on background. Text=foreground,
TextDisabled=dark_foreground, Border/Separator=**muted**,
TextSelectedBg=**selection**, TabSelected=mix(selection, foreground, 0.04),
ModalWindowDimBg/NavWindowingDimBg over darker_background,
ScrollbarGrab=muted, SliderGrab=mix(accent, frame, 0.25).

Shape: `WindowRounding = WindowBorderSize = ChildRounding = TabBorderSize = 0`
(Hyprland draws corners/borders); `FrameRounding/GrabRounding/TabRounding =
round(base·0.16)`, `PopupRounding = round(base·0.20)`,
`ScrollbarRounding = round(base·0.45)`, `FrameBorderSize = 1`,
spacings = `ItemSpacing(6,4)·scale`, `FramePadding(8, base·0.25)`,
`WindowPadding(10,8)·scale`, `ScrollbarSize = round(base·0.9)`,
`GrabMinSize = round(base·0.6)`.

## 7. Font & background

* Font: fontconfig is the source of truth (`fc-match monospace` family first
  entry, file via `%{file}`) — `omarchy font set` reconfigures fontconfig.
* Background: target of `current/background`; it may be a relative symlink
  (resolve against the link's directory).

## 8. Versioning

* This contract: additions are compatible; the conformance vector set must
  be regenerated (and adapters re-passed) if resolution changes.
* C ABI (§ `bindings/c/omarchy_theme.h`): struct is frozen at
  `OMARCHY_ABI_VERSION = 1`, 1560 bytes, `colors` at 88, `style` at 344.
  Slot arrays are append-only *within* the reserved regions; renaming or
  moving bumps the version.
