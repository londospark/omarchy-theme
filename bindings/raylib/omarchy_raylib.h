/* omarchy_raylib.h — raylib helpers over the omarchy-theme C ABI.
 *
 * raylib has no widget style system, so this adapter is deliberately small:
 * color conversion, the one-line poll-and-reload, oracle-identical mixes,
 * and wallpaper loading. Everything works straight off the POD struct from
 * omarchy_theme.h — link libomarchy_theme_abi and include both headers.
 *
 * raylib's Color {u8 r,g,b,a} is byte-identical to the 0xRRGGBBAA packing
 * used by OmarchyTheme.colors, so conversions are pure field moves.
 */
#ifndef OMARCHY_RAYLIB_H
#define OMARCHY_RAYLIB_H

#include "omarchy_theme.h"
#include "raylib.h"

#ifdef __cplusplus
extern "C" {
#endif

/* Convert a palette slot to a raylib Color (alpha 0 if unavailable). */
static inline Color omarchy_rl(const OmarchyTheme *t, OmarchyColorSlot slot) {
    uint32_t v = t->colors[slot];
    Color c;
    c.r = (unsigned char)((v >> 24) & 0xFF);
    c.g = (unsigned char)((v >> 16) & 0xFF);
    c.b = (unsigned char)((v >> 8) & 0xFF);
    c.a = (unsigned char)(v & 0xFF);
    return c;
}

/* ANSI slot 0..15. */
static inline Color omarchy_rl_ansi(const OmarchyTheme *t, int index) {
    return omarchy_rl(t, (OmarchyColorSlot)OMC_ANSI(index));
}

/* Poll tier: reload into `t` when the desktop theme changed. Returns true
 * when `t` now holds fresh data. One line per frame. */
static inline int omarchy_rl_update(OmarchyTheme *t) {
    if (!omarchy_theme_changed()) return 0;
    return omarchy_theme_load(t) == OMARCHY_OK;
}

/* Per-channel integer mix with half-up rounding (visually equivalent to
 * omarchy-theme-color's float `mix` helper, which this cannot call).
 * `amount` 0..255 (255 = fully `to`). */
static inline Color omarchy_rl_mix(Color from, Color to, int amount) {
    int a = amount < 0 ? 0 : (amount > 255 ? 255 : amount);
    Color out;
    out.r = (unsigned char)((from.r * (255 - a) + to.r * a + 127) / 255);
    out.g = (unsigned char)((from.g * (255 - a) + to.g * a + 127) / 255);
    out.b = (unsigned char)((from.b * (255 - a) + to.b * a + 127) / 255);
    out.a = (unsigned char)((from.a * (255 - a) + to.a * a + 127) / 255);
    return out;
}

/* Shell control-state fill: `layer` over `surface` at a 0..1 alpha from
 * shell.toml [controls] (e.g. OMS_HOVER_FILL_ALPHA). */
static inline Color omarchy_rl_fill(Color surface, Color layer, float alpha) {
    return omarchy_rl_mix(surface, layer, (int)(alpha * 255.0f + 0.5f));
}

/* Load the active wallpaper (caller unloads). Returns an empty texture when
 * no background is set or the load fails. */
static inline Texture2D omarchy_rl_wallpaper(const OmarchyTheme *t) {
    Texture2D empty = {0, 0, 0, 0, 0};
    if (!t->has_background) return empty;
    if (t->background_path[0] == '\0') return empty;
    return LoadTexture(t->background_path);
}

/* Clear to the theme background at an optional brightness offset
 * (delta -255..255 shifts all channels; useful for canvas vs chrome). */
static inline void omarchy_rl_clear(const OmarchyTheme *t, int delta) {
    Color bg = omarchy_rl(t, OMC_BACKGROUND);
    Color out = {
        (unsigned char)(bg.r + delta < 0 ? 0 : (bg.r + delta > 255 ? 255 : bg.r + delta)),
        (unsigned char)(bg.g + delta < 0 ? 0 : (bg.g + delta > 255 ? 255 : bg.g + delta)),
        (unsigned char)(bg.b + delta < 0 ? 0 : (bg.b + delta > 255 ? 255 : bg.b + delta)),
        255
    };
    ClearBackground(out);
}

#ifdef __cplusplus
}
#endif

#endif /* OMARCHY_RAYLIB_H */
