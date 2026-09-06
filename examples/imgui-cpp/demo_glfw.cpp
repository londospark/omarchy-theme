// The live-window half of the imgui-cpp demo: an ordinary GLFW/OpenGL3 app
// whose only theming line is `omarchy::AutoApply(ImGui::GetStyle())` at the
// top of each frame. Switch the desktop theme and watch it follow.
#include <cstdio>
#include <GL/gl.h>
#include <GLFW/glfw3.h>

#include "backends/imgui_impl_glfw.h"
#include "backends/imgui_impl_opengl3.h"
#include "imgui.h"
#include <omarchy/imgui.hpp>

int omarchy_demo_gui_run() {
    if (!glfwInit()) { fprintf(stderr, "glfw init failed\n"); return 1; }
    GLFWwindow* win = glfwCreateWindow(900, 600, "omarchy-theme x Dear ImGui", nullptr, nullptr);
    if (!win) { glfwTerminate(); return 1; }
    glfwMakeContextCurrent(win);
    glfwSwapInterval(1);

    ImGui::CreateContext();
    ImGuiIO& io = ImGui::GetIO();
    io.ConfigFlags |= ImGuiConfigFlags_DockingEnable;
    ImGui_ImplGlfw_InitForOpenGL(win, true);
    ImGui_ImplOpenGL3_Init("#version 330");

    // Font: prefer the live Omarchy UI font file when resolvable.
    omarchy::Theme theme;
    float font_size = 18.0f;
    if (theme.loaded && !theme.fontPath.empty()) {
        io.Fonts->AddFontFromFileTTF(theme.fontPath.c_str(), font_size);
    } else {
        io.Fonts->AddFontDefault();
    }

    float f = 3.14f;
    int n = 42;
    bool check = true;
    float color[4] = {0.5f, 0.5f, 0.5f, 1.0f};
    char buf[128] = "hello omarchy";

    while (!glfwWindowShouldClose(win)) {
        glfwPollEvents();
        ImGui_ImplOpenGL3_NewFrame();
        ImGui_ImplGlfw_NewFrame();
        ImGui::NewFrame();

        // ---- the only theming the app writes: one line per frame ----
        omarchy::AutoApply(ImGui::GetStyle());

        ImGui::Begin("Widget tour");
        ImGui::Text("Following Omarchy theme: %s",
                    theme.loaded ? theme.name.c_str() : "(pending)");
        ImGui::Separator();
        ImGui::InputText("text", buf, IM_ARRAYSIZE(buf));
        ImGui::SliderFloat("slider", &f, 0.0f, 10.0f);
        ImGui::DragInt("drag", &n, 1, 0, 100);
        ImGui::Checkbox("checkbox", &check);
        ImGui::ColorEdit4("color", color);
        if (ImGui::Button("button")) {}
        ImGui::ProgressBar(f / 10.0f);
        if (ImGui::BeginCombo("combo", "option")) {
            for (const char* item : {"option", "second", "third"})
                if (ImGui::Selectable(item)) {}
            ImGui::EndCombo();
        }
        if (ImGui::BeginTabBar("tabs")) {
            if (ImGui::BeginTabItem("Tables")) {
                static ImGuiTableFlags tf = ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg;
                if (ImGui::BeginTable("t", 3, tf)) {
                    ImGui::TableSetupColumn("Key");
                    ImGui::TableSetupColumn("Value");
                    ImGui::TableSetupColumn("Accent");
                    ImGui::TableHeadersRow();
                    for (auto& [k, v] : theme.palette.entries) {
                        ImGui::TableNextRow();
                        ImGui::TableNextColumn(); ImGui::Text("%s", k.c_str());
                        ImGui::TableNextColumn(); ImGui::Text("%s", v.c_str());
                        ImGui::TableNextColumn(); ImGui::TextColored(theme.Accent(), "#");
                        if (ImGui::GetTableRowIndex() > 40) break;
                    }
                    ImGui::EndTable();
                }
                ImGui::EndTabItem();
            }
            if (ImGui::BeginTabItem("Log")) {
                ImGui::TextWrapped("Switch the desktop theme (omarchy theme set "
                                   "tokyo-night) and this window retints live.");
                ImGui::EndTabItem();
            }
            ImGui::EndTabBar();
        }
        ImGui::End();

        // fullscreen dockspace so tabs/widgets show the theme everywhere
        ImGui::DockSpaceOverViewport(0, ImGui::GetMainViewport(), 0);

        ImGui::Render();
        int w, h;
        glfwGetFramebufferSize(win, &w, &h);
        glViewport(0, 0, w, h);
        glClearColor(theme.Background().r * 0.6f, theme.Background().g * 0.6f,
                     theme.Background().b * 0.6f, 1.0f);
        glClear(GL_COLOR_BUFFER_BIT);
        ImGui_ImplOpenGL3_RenderDrawData(ImGui::GetDrawData());
        glfwSwapBuffers(win);
    }

    ImGui_ImplOpenGL3_Shutdown();
    ImGui_ImplGlfw_Shutdown();
    ImGui::DestroyContext();
    glfwDestroyWindow(win);
    glfwTerminate();
    return 0;
}
