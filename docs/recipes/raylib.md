# Recipe: raylib (C)

Everything lives in two headers:
[`bindings/c/omarchy_theme.h`](../../bindings/c/omarchy_theme.h) (frozen ABI)
and [`bindings/raylib/omarchy_raylib.h`](../../bindings/raylib/omarchy_raylib.h).
raylib's `Color {u8 r,g,b,a}` is byte-identical to the struct's `0xRRGGBBAA`
slots, so conversion is a field move.

```c
#include <omarchy_theme.h>
#include <omarchy_raylib.h>

OmarchyTheme t = {0};
t.abi_size = (uint32_t)sizeof t;
omarchy_theme_load(&t);
Font font = LoadFontEx(t.font_path[0] ? t.font_path : DEFAULT_FONT, 24, NULL, 0);

while (!WindowShouldClose()) {
    if (omarchy_rl_update(&t)) { /* reload font if you care */ }
    BeginDrawing();
        omarchy_rl_clear(&t, 0);
        // interaction-tinted fill exactly like the shell's controls:
        Color base  = omarchy_rl(&t, OMC_LIGHTER_BACKGROUND);
        Color hover = omarchy_rl_fill(base, omarchy_rl(&t, OMC_ACCENT),
                                      t.style[OMS_HOVER_FILL_ALPHA]);
        DrawRectangle(40, 40, 200, 44, hover);
        DrawTextEx(font, "live omarchy theme", (Vector2){56, 52}, 24, 1,
                   omarchy_rl(&t, OMC_FOREGROUND));
    EndDrawing();
}
```

Wallpaper: `Texture2D bg = omarchy_rl_wallpaper(&t);` (`current/background`).

Build (library from this workspace, or installed):

```sh
cargo build --release -p omarchy-theme-abi
cc game.c -Ibindings/c -Ibindings/raylib -Ltarget/release \
   -lomarchy_theme_abi -lraylib -lm -o game
```

Runnable example: [`examples/raylib-c/main.c`](../../examples/raylib-c/main.c)
(HUD with ANSI strip + control-state fills; compile-checked in CI via
`-fsyntax-only` against vendored raylib headers — see
[examples/README.md](../../examples/README.md)).
