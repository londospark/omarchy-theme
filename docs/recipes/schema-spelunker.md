# Recipe: Schema-Spelunker (Odin + dcimgui)

[Schema-Spelunker](https://github.com/londospark/Schema-Spelunker) already
has the two halves this framework assumes an app owns: a flat `ThemeData`
struct + `apply_theme()` (ImGui/ImNodes style), and a line-parsed `.ssTheme`
loader (`parse_ssTheme`) with folder-scan theme discovery. What it lacks is a
bridge from the desktop theme to `ThemeData`. This recipe adds it — pure
Odin, no new binary, no C library, ~2 lines of wiring.

## Files

* [`examples/schema-spelunker/omarchy_theme_style.odin`](../examples/schema-spelunker/omarchy_theme_style.odin)
  — the bridge: state-dir reading, the contract's poll rule, and a
  mapping-v1 faithful `ThemeData` producer.
* [`examples/schema-spelunker/demo.odin`](../examples/schema-spelunker/demo.odin)
  — standalone driver showing output values; run it with
  `odin run examples/schema-spelunker -out:demo` from this repo.

## Wiring (PR-ready shape)

**1. `style.odin` additions** — copy `omarchy_theme_style.odin` next to it
into `package main`, then adapt `theme_data_from_omarchy`'s field names to
`ThemeData` (they are already the same names; drop `Omarchy_ImGui_Theme` and
return `ThemeData` directly). Suggested values:

| ThemeData field | Omarchy source |
|---|---|
| `text_main` | `foreground` |
| `text_muted` | `dark_foreground` |
| `bg_window/child` | `background` @ `backdrop_alpha` |
| `bg_popup` | `background` @ `[popups] background-alpha` (1.0 default) |
| `ctrl_frame` | `mix(background, lighter_background, 0.55)` |
| `ctrl_frame_hover` | `mix(frame, foreground, hover-cursor-fill-alpha)` |
| `ctrl_frame_active` | `mix(frame, accent, pressed-fill-alpha)` |
| `title_bg / focus / faded` | `darker_background / background / dark_background` |
| `card_bg / card_outline` | `lighter_background / muted` |
| `border_main / subtle` | `muted / selection` |
| `grid_bg / grid_line` | `dark_background / darker_background` |
| `link_color / accent_colour` | `blue / accent` (or both accent) |
| `corner_rounding …` | keep app defaults; rounding is theme-independent today |

**2. `main.odin` startup** — where the app currently calls `apply_theme`:

```odin
theme_sig := ""
if k, ok := load_omarchy_keys(); ok {
    data := theme_data_from_omarchy(&k)
    apply_theme(data)
}
```

**3. frame top** (the ImGui NewFrame section of your loop):

```odin
if omarchy_theme_changed(&theme_sig, context.temp_allocator) {
    if k, ok := load_omarchy_keys(); ok {
        apply_theme(theme_data_from_omarchy(&k))
        reload_font_from_omarchy()   // optional: fc-match font via load_omarchy font
    }
}
```

That's it: switch to tokyo-night from the menu and the spelunker retints
before you finish alt-tabbing.

**Optional: an "Omarchy (live)" item in the theme menu.** Call
`write_omarchy_ss_theme("assets/themes/omarchy_live.ssTheme", "Omarchy (live)")`
from a `theme-set.d` hook (see [`hook/omarchy-theme-gen`](../../hook/omarchy-theme-gen))
and the app's existing `discover_themes()` will list it with no code change —
users can then opt in explicitly instead of auto-follow.

## Font

`load_omarchy_keys()` resolves the palette; for the font, spawn once:
`fc-match monospace -f '%{family}\n%{file}\n'` (see contract §7) and pass
`%{file}` to your existing `io.Fonts->AddFontFromFileTTF` path — the same
file the desktop bar and terminals are using.

## Verification

The bridge's resolver (not just its mapping) is already proven: it is a copy
of the same cascade validated by `examples/odin-conformance/` against every
conformance vector. If you port the struct mapping and the numbers still look
wrong after a theme switch, the bug is in the mapping table above — fix it
upstream here so every app benefits.
