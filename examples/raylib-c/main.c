/* omarchy-theme raylib example: a HUD that live-follows the desktop theme.
 *
 * Build (needs raylib installed, plus the workspace's C ABI library):
 *   cargo build --release -p omarchy-theme-abi
 *   cc -I../../bindings/c -I../../bindings/raylib main.c \
 *      -L../../target/release -lomarchy_theme_abi -lraylib -lm -o demo
 *
 * Requires only the headers + link flags; no Rust at the call site.
 */
#include <stddef.h>

#include <omarchy_theme.h>
#include <omarchy_raylib.h>

int main(void) {
    InitWindow(800, 480, "omarchy-theme x raylib");
    SetTargetFPS(60);

    OmarchyTheme t;
    t.abi_size = (uint32_t)sizeof t;
    omarchy_theme_load(&t);

    Font font = LoadFontEx(t.font_path[0] ? t.font_path : "", 28, NULL, 0);

    while (!WindowShouldClose()) {
        if (omarchy_rl_update(&t)) {           /* one line: follow the desktop */
            UnloadFont(font);
            font = LoadFontEx(t.font_path[0] ? t.font_path : "", 28, NULL, 0);
        }

        BeginDrawing();
        omarchy_rl_clear(&t, 0);

        Color accent = omarchy_rl(&t, OMC_ACCENT);
        Color fg = omarchy_rl(&t, OMC_FOREGROUND);
        Color surface = omarchy_rl(&t, OMC_LIGHTER_BACKGROUND);

        DrawTextEx(font, "Schema-Spelunker style: live Omarchy theme",
                   (Vector2){40, 30}, 28, 1, fg);

        /* shell control-state fills: hover/pressed read like the bar does */
        float hoverA = t.style[OMS_HOVER_FILL_ALPHA];
        float pressA = t.style[OMS_PRESSED_FILL_ALPHA];
        DrawRectangle(40, 90, 200, 40, omarchy_rl_fill(surface, accent, pressA));
        DrawRectangle(260, 90, 200, 40, omarchy_rl_fill(surface, accent, hoverA));
        DrawText("pressed", 110, 102, 20, fg);
        DrawText("hover", 345, 102, 20, fg);

        /* ANSI strip, like a terminal preview */
        for (int i = 0; i < 16; i++) {
            DrawRectangle(40 + i * 44, 160, 40, 40, omarchy_rl_ansi(&t, i));
        }

        DrawText(t.theme_name, 40, 230, 20, accent);
        EndDrawing();
    }

    UnloadFont(font);
    CloseWindow();
    return 0;
}
