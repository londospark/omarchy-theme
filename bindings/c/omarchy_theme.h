/* omarchy_theme.h — C ABI for the omarchy-theme resolver.
 *
 * Layout is frozen per OMARCHY_ABI_VERSION: the struct size (1560 bytes on
 * any ILP32/LP64 platform with 4-byte alignment), the colors offset (88),
 * and the style offset (344) are asserted by a Rust test
 * (crates/omarchy-theme-abi) and by the conformance bindings.
 *
 * Link against libomarchy_theme_abi.so/.a (built with
 * `cargo build --release -p omarchy-theme-abi`; note the underscored file name). No threads are spawned by
 * these calls; the library allocates only an internal cache.
 *
 * Colors are packed 0xRRGGBBAA — byte-identical to raylib's Color, ImNodes'
 * u32 colors, and ImGui's Color4-u32 convention. A slot whose value could
 * not be parsed holds 0 (alpha == 0 means "unavailable"). Style floats are
 * NaN when a token is unavailable.
 */
#ifndef OMARCHY_THEME_H
#define OMARCHY_THEME_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define OMARCHY_ABI_VERSION 1u

typedef enum OmarchyColorSlot {
    OMC_ACCENT = 0,
    OMC_SELECTION = 1,
    OMC_MUTED = 2,
    OMC_BACKGROUND = 3,
    OMC_DARK_BACKGROUND = 4,
    OMC_DARKER_BACKGROUND = 5,
    OMC_LIGHTER_BACKGROUND = 6,
    OMC_FOREGROUND = 7,
    OMC_DARK_FOREGROUND = 8,
    OMC_LIGHT_FOREGROUND = 9,
    OMC_BRIGHT_FOREGROUND = 10,
    OMC_CURSOR = 11,
    OMC_SELECTION_BACKGROUND = 12,
    OMC_SELECTION_FOREGROUND = 13,
    OMC_RED = 14,
    OMC_YELLOW = 15,
    OMC_ORANGE = 16,
    OMC_GREEN = 17,
    OMC_CYAN = 18,
    OMC_BLUE = 19,
    OMC_MAGENTA = 20,
    OMC_BROWN = 21,
    OMC_PURPLE = 22,
    OMC_BRIGHT_RED = 23,
    OMC_BRIGHT_YELLOW = 24,
    OMC_BRIGHT_GREEN = 25,
    OMC_BRIGHT_CYAN = 26,
    OMC_BRIGHT_BLUE = 27,
    OMC_BRIGHT_MAGENTA = 28,
    OMC_BRIGHT_PURPLE = 29,
    OMC_ACTIVE_BORDER = 30 /* Hyprland active-window border, flattened */
    /* 31 reserved; 32..47 = ANSI color0..15 */
} OmarchyColorSlot;

#define OMC_ANSI(n) (32 + (n)) /* n in 0..15 */
#define OMC_ANSI0 OMC_ANSI(0)

typedef enum OmarchyStyleSlot {
    OMS_NORMAL_FILL_ALPHA = 0,
    OMS_NORMAL_BORDER_ALPHA = 1,
    OMS_NORMAL_BORDER_WIDTH = 2,
    OMS_HOVER_FILL_ALPHA = 3,
    OMS_HOVER_BORDER_ALPHA = 4,
    OMS_HOVER_BORDER_WIDTH = 5,
    OMS_FOCUS_FILL_ALPHA = 6,
    OMS_FOCUS_BORDER_ALPHA = 7,
    OMS_FOCUS_BORDER_WIDTH = 8,
    OMS_SELECTED_FILL_ALPHA = 9,
    OMS_SELECTED_BORDER_ALPHA = 10,
    OMS_SELECTED_BORDER_WIDTH = 11,
    OMS_PRESSED_FILL_ALPHA = 12,
    OMS_SELECTION_FILL_ALPHA = 13,
    OMS_SPACING_SCALE = 14,
    OMS_FONT_BASE_SIZE = 15,
    OMS_MENU_BG_ALPHA = 16,
    OMS_POPUP_BG_ALPHA = 17,
    OMS_TOOLTIP_BG_ALPHA = 18,
    OMS_SCRIM_ALPHA = 19
    /* 20..31 reserved */
} OmarchyStyleSlot;

typedef struct OmarchyTheme {
    uint32_t abi_size;        /* [in]  sizeof(OmarchyTheme) before load() */
    uint32_t abi_version;     /* [out] */
    int32_t  mode;            /* [out] 0 = dark, 1 = light */
    uint32_t has_font;
    uint32_t has_background;
    uint32_t reserved0;
    char     theme_name[64];  /* slug, e.g. "catppuccin" */
    uint32_t colors[64];      /* 0xRRGGBBAA, indexed by OmarchyColorSlot */
    float    style[32];       /* OmarchyStyleSlot; NaN when unavailable */
    char     font_family[64]; /* fontconfig-resolved monospace UI font */
    char     font_path[512];  /* loadable font file ("" if unavailable) */
    char     background_path[512]; /* active wallpaper ("" if none) */
} OmarchyTheme;

/* Usage:
 *   OmarchyTheme t;
 *   memset(&t, 0, sizeof t);
 *   t.abi_size = (uint32_t)sizeof t;
 *   if (omarchy_theme_load(&t) == OMARCHY_OK) { ... use t.colors[] ... }
 *   // once per frame:
 *   if (omarchy_theme_changed()) omarchy_theme_load(&t);
 */

/* Error codes */
#define OMARCHY_OK            0
#define OMARCHY_ERR          (-1)
#define OMARCHY_ERR_NO_THEME (-2)
#define OMARCHY_ERR_ABI      (-3)
#define OMARCHY_ERR_ARG      (-4)

/* ABI implemented by the linked library. */
uint32_t omarchy_theme_abi_version(void);
/* Library version string ("x.y.z"), valid for process lifetime. */
const char *omarchy_theme_version(void);
/* Load the current theme; returns OMARCHY_OK or a negative code. */
int32_t omarchy_theme_load(OmarchyTheme *out);
/* 1 = desktop theme changed since last successful load. Cheap poll. */
int32_t omarchy_theme_changed(void);
/* Resolved palette value by key from the last load (raw string form).
 * Returns value length excluding NUL, or a negative code. */
int32_t omarchy_theme_get(const char *key, char *out, uint32_t out_len);
/* Enumerate the full resolved palette (sorted by key). */
uint32_t omarchy_theme_entry_count(void);
int32_t omarchy_theme_entry(uint32_t index, char *key, uint32_t key_len,
                            char *value, uint32_t value_len);

#ifdef __cplusplus
} /* extern "C" */
#endif

#endif /* OMARCHY_THEME_H */
