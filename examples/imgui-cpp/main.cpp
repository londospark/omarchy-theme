// omarchy-theme x Dear ImGui demo (headless smoke + manual window).
//
// Live demo:   ./build.sh run        (opens an ImGui window; change the
//              desktop theme with `omarchy theme set <name>` — the window
//              retints on the next frame, no restart.)
// Smoke test:  ./build.sh test       (asserts the applied style matches the
//              live palette, then exits 0/1; nothing is displayed.)
//
// Only ImGui core sources are needed for the smoke test; the `run` mode
// adds the GLFW/OpenGL3 backends.
#include <cctype>
#include <cmath>
#include <cstdio>
#include <cstring>

#include "imgui.h"
#include <omarchy/imgui.hpp>

int main(int argc, char** argv) {
    bool test_mode = argc > 1 && !strcmp(argv[1], "--test");

    ImGui::CreateContext();
    ImGuiStyle& style = ImGui::GetStyle();

    // The one-liner: load + apply now, and again on every theme change.
    omarchy::AutoApply(style);

    omarchy::Theme theme;  // independent copy for assertions
    if (!theme.loaded) {
        fprintf(stderr, "no live omarchy theme found\n");
        return test_mode ? 1 : 0;
    }

    if (test_mode) {
        // Spot-check applied colors against the palette (float4 vs hex).
        int fails = 0;
        // spot-checks: Text == foreground, TextSelectedBg == selection,
        // Border == muted
        const omarchy::Rgba fg = theme.palette.Color("foreground");
        const omarchy::Rgba sel = theme.palette.Color("selection");
        const omarchy::Rgba muted = theme.palette.Color("muted");
        auto cmp = [&](const ImVec4& got, const omarchy::Rgba& want, const char* what) {
            bool ok = fabsf(got.x - want.r) < 0.01f && fabsf(got.y - want.g) < 0.01f
                   && fabsf(got.z - want.b) < 0.01f;
            if (!ok) fprintf(stderr, "%s mismatch: got (%.3f,%.3f,%.3f) want (%.3f,%.3f,%.3f)\n",
                              what, got.x, got.y, got.z, want.r, want.g, want.b);
            return ok ? 0 : 1;
        };
        fails = cmp(style.Colors[ImGuiCol_Text], fg, "Text")
              + cmp(style.Colors[ImGuiCol_TextSelectedBg], sel, "TextSelectedBg")
              + cmp(style.Colors[ImGuiCol_Border], muted, "Border");

        // scalars pinned by the recipe
        if (style.WindowBorderSize != 0.0f) {
            fprintf(stderr, "WindowBorderSize should be 0 (compositor draws borders)\n");
            fails++;
        }
        if (style.WindowRounding != 0.0f) {
            fprintf(stderr, "WindowRounding should be 0 (compositor rounds)\n");
            fails++;
        }
        if (fails) { printf("imgui-cpp smoke: FAIL (%d)\n", fails); return 1; }
        printf("imgui-cpp smoke: ok (theme %s)\n", theme.name.c_str());
        ImGui::DestroyContext();
        return 0;
    }

#ifdef OMARCHY_DEMO_GUI
    extern int omarchy_demo_gui_run();  // demo_glfw.cpp
    int rc = omarchy_demo_gui_run();
    ImGui::DestroyContext();
    return rc;
#else
    printf("built headless; rebuild with ./build.sh run for the window demo\n");
    ImGui::DestroyContext();
    return 0;
#endif
}
