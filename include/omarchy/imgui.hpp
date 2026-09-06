// omarchy/imgui.hpp — header-only runtime adapter: follows the live Omarchy
// desktop theme and applies it to a Dear ImGui style. No build-system
// integration, no Rust, no codegen: read state, parse, apply, poll.
//
//   #include <omarchy/imgui.hpp>
//   static omarchy::Theme theme;                       // loads current
//   omarchy::ApplyStyle(ImGui::GetStyle(), theme);     // at startup
//   ...
//   if (theme.ReloadIfChanged())                       // once per frame
//       omarchy::ApplyStyle(ImGui::GetStyle(), theme);
//
// Or the one-liner: omarchy::AutoApply(ImGui::GetStyle());
//
// Requires C++17, Dear ImGui (vanilla 1.89+ or docking; docking-only colors
// are set defensively by member index name via #ifdef). The palette cascade
// mirrors crates/omarchy-theme/src/palette.rs and omarchy-theme-color; the
// color recipe mirrors imgui_recipe.rs ("mapping v1"). Keep all three in
// sync; docs/contract.md is the normative spec.

#ifndef OMARCHY_IMGUI_HPP
#define OMARCHY_IMGUI_HPP

#include <imgui.h>

#include <algorithm>
#include <cctype>
#include <cmath>
#include <cstdio>
#include <cstring>
#include <fstream>
#include <map>
#include <sstream>
#include <string>
#include <vector>

namespace omarchy {

// ---------------------------------------------------------------------------
// colors
// ---------------------------------------------------------------------------

struct Rgba {
    float r = 0, g = 0, b = 0, a = 1;

    operator ImVec4() const { return ImVec4(r, g, b, a); }

    static Rgba Hex(const std::string& s) {
        Rgba c;
        if (s.empty() || s[0] != '#') return c;
        size_t n = s.size() - 1;
        if (n != 3 && n != 6 && n != 8) return c;
        auto d = [&](size_t i) -> int {
            char x = (char)std::tolower((unsigned char)s[1 + i]);
            if (x >= '0' && x <= '9') return x - '0';
            if (x >= 'a' && x <= 'f') return x - 'a' + 10;
            return -1;
        };
        auto pair = [&](size_t i) -> int { return d(i) * 16 + d(i + 1); };
        if (n == 3) {
            c.r = d(0) * 17 / 255.f; c.g = d(1) * 17 / 255.f; c.b = d(2) * 17 / 255.f;
        } else {
            int r0 = pair(0), g0 = pair(2), b0 = pair(4);
            if (r0 < 0 || g0 < 0 || b0 < 0) return Rgba{};
            c.r = r0 / 255.f; c.g = g0 / 255.f; c.b = b0 / 255.f;
            if (n == 8) c.a = pair(6) / 255.f;
        }
        return c;
    }

    static int HexByte(char x) {
        x = (char)std::tolower((unsigned char)x);
        if (x >= '0' && x <= '9') return x - '0';
        if (x >= 'a' && x <= 'f') return x - 'a' + 10;
        return 0;
    }
    static int Pair(const std::string& s, size_t i) {
        return HexByte(s[i]) * 16 + HexByte(s[i + 1]);
    }

    // Oracle-compatible mix: int(start*(1-a)+end*a+0.5) per channel.
    Rgba Mix(const Rgba& other, float amount) const {
        if (amount > 1.f) amount /= 100.f;
        amount = std::clamp(amount, 0.f, 1.f);
        auto ch = [&](float s, float d) {
            int si = (int)std::lround(s * 255.f);
            int di = (int)std::lround(d * 255.f);
            float f = (float)(int)(si * (1.0 - amount) + di * amount + 0.5);
            return f / 255.f;
        };
        return {ch(r, other.r), ch(g, other.g), ch(b, other.b), ch(a, other.a)};
    }

    Rgba Alpha(float a) const { Rgba c = *this; c.a = std::clamp(a, 0.f, 1.f); return c; }

    std::string HexOut() const {
        char buf[8];
        std::snprintf(buf, sizeof buf, "#%02x%02x%02x",
                      (int)(r * 255.f + 0.5f), (int)(g * 255.f + 0.5f), (int)(b * 255.f + 0.5f));
        return buf;
    }

    // Oracle light/dark test: r+g+b (bytes) > 382.
    bool IsLight() const {
        int ri = (int)(r * 255.f + 0.5f), gi = (int)(g * 255.f + 0.5f), bi = (int)(b * 255.f + 0.5f);
        return ri + gi + bi > 382;
    }
};

// ---------------------------------------------------------------------------
// palette (colors.toml + resolution cascade)
// ---------------------------------------------------------------------------

class Palette {
public:
    std::map<std::string, std::string> entries;

    static Palette FromFile(const std::string& path, bool light_marker) {
        Palette p;
        std::ifstream in(path);
        if (!in) return p;
        std::string line;
        while (std::getline(in, line)) {
            size_t eq = line.find('=');
            if (eq == std::string::npos) continue;
            std::string key;
            for (size_t i = 0; i < eq; ++i) {
                char c = line[i];
                if (c != '"' && c != '\'' && c != ' ' && c != '\t') key += c;
            }
            if (key.empty() || key[0] == '#') continue;
            for (char c : key)
                if (!(std::isalnum((unsigned char)c) || c == '_' || c == '-')) { key.clear(); break; }
            if (key.empty()) continue;
            std::string value = ExtractValue(line.substr(eq + 1));
            for (char c : value) {
                if (!(std::isalnum((unsigned char)c) ||
                      std::strchr("#(),._+/% -", c) != nullptr)) {
                    value.clear();
                    key.clear();
                    break;
                }
            }
            if (!value.empty() || key.empty() == false)
                p.entries[key] = value; // last definition wins, like the oracle
        }
        p.Resolve(light_marker);
        return p;
    }

    const std::string* Get(const std::string& key) const {
        auto it = entries.find(key);
        if (it == entries.end() || it->second.empty()) return nullptr;
        return &it->second;
    }

    std::string Str(const std::string& key, const std::string& fallback = "") const {
        const std::string* v = Get(key);
        return v ? *v : fallback;
    }

    Rgba Color(const std::string& key) const {
        const std::string* v = Get(key);
        return v ? Rgba::Hex(*v) : Rgba{};
    }

    Rgba Ansi(int i) const { return Color("color" + std::to_string(i)); }

    bool IsLight() const { return mode_ == "light"; }
    const std::string& Mode() const { return mode_; }

private:
    std::string mode_ = "dark";

    static std::string ExtractValue(std::string v) {
        size_t q = v.find_first_of("'\"");
        if (q != std::string::npos) {
            size_t e = v.find_first_of("'\"", q + 1);
            if (e == std::string::npos) e = v.size();
            return v.substr(q + 1, e - q - 1);
        }
        size_t a = v.find_first_not_of(" \t\r\n");
        if (a == std::string::npos) return "";
        size_t b = v.find_last_not_of(" \t\r\n");
        return v.substr(a, b - a + 1);
    }

    void Alias(const std::string& key, const std::string& fallback) {
        if (Get(key)) return;
        const std::string* v = Get(fallback);
        if (v) entries[key] = *v;
    }

    Rgba Need(const std::string& key) const { return Color(key); }

    void Resolve(bool light_marker) {
        static const std::pair<const char*, const char*> kLegacy[] = {
            {"background", "bg"}, {"dark_background", "dark_bg"},
            {"darker_background", "darker_bg"}, {"lighter_background", "lighter_bg"},
            {"foreground", "fg"}, {"dark_foreground", "dark_fg"},
            {"light_foreground", "light_fg"}, {"bright_foreground", "bright_fg"},
        };
        for (auto& [k, f] : kLegacy) Alias(k, f);

        if (!Get("background")) Alias("background", "color0");
        if (!Get("foreground")) Alias("foreground", "color7");
        if (Get("background")) entries["color0"] = *Get("background");
        if (Get("foreground")) entries["color7"] = *Get("foreground");

        static const std::pair<const char*, const char*> kAnsiSemantic[] = {
            {"red", "color1"}, {"green", "color2"}, {"yellow", "color3"},
            {"blue", "color4"}, {"magenta", "color5"}, {"cyan", "color6"},
            {"bright_red", "color9"}, {"bright_green", "color10"},
            {"bright_yellow", "color11"}, {"bright_blue", "color12"},
            {"bright_magenta", "color13"}, {"bright_cyan", "color14"},
        };
        for (auto& [k, f] : kAnsiSemantic) Alias(k, f);
        Alias("magenta", "purple");
        Alias("bright_magenta", "bright_purple");

        if (!Get("light_foreground"))
            if (const std::string* v = Get("color7") ? Get("color7") : Get("foreground"))
                entries["light_foreground"] = *v;
        if (!Get("bright_foreground"))
            if (const std::string* v = Get("color15") ? Get("color15") : Get("foreground"))
                entries["bright_foreground"] = *v;
        entries["cursor"] = Str("bright_foreground");
        if (!Get("lighter_background"))
            if (const std::string* v = Get("color0") ? Get("color0") : Get("background"))
                entries["lighter_background"] = *v;
        if (!Get("dark_foreground"))
            if (const std::string* v = Get("color8") ? Get("color8") : Get("foreground"))
                entries["dark_foreground"] = *v;
        if (!Get("muted"))
            if (const std::string* v = Get("color8") ? Get("color8") : Get("dark_foreground"))
                entries["muted"] = *v;
        if (!Get("selection")) {
            for (const char* k : {"selection_background", "color8", "color0", "background"})
                if (const std::string* v = Get(k)) { entries["selection"] = *v; break; }
        }
        if (!Get("selection_background")) Alias("selection_background", "selection");
        if (!Get("selection_foreground")) Alias("selection_foreground", "bright_foreground");
        Alias("orange", "yellow");
        if (!Get("brown"))
            entries["brown"] = Need("orange").Mix(Rgba{}, 0.5f).HexOut();

        if (!Get("dark_background"))
            entries["dark_background"] = Need("background").Mix(Rgba{}, 0.25f).HexOut();
        if (!Get("darker_background"))
            entries["darker_background"] = Need("background").Mix(Rgba{}, 0.5f).HexOut();
        static const std::pair<const char*, const char*> kBright[] = {
            {"bright_red", "red"}, {"bright_yellow", "yellow"}, {"bright_green", "green"},
            {"bright_cyan", "cyan"}, {"bright_blue", "blue"}, {"bright_magenta", "magenta"},
        };
        for (auto& [k, base] : kBright)
            if (!Get(k)) entries[k] = Need(base).Mix(Lit(), 0.2f).HexOut();
        Alias("purple", "magenta");
        Alias("bright_purple", "bright_magenta");

        static const std::pair<const char*, const char*> kSemanticAnsi[] = {
            {"color0", "background"}, {"color1", "red"}, {"color2", "green"},
            {"color3", "yellow"}, {"color4", "blue"}, {"color5", "magenta"},
            {"color6", "cyan"}, {"color7", "foreground"}, {"color8", "muted"},
            {"color9", "bright_red"}, {"color10", "bright_green"},
            {"color11", "bright_yellow"}, {"color12", "bright_blue"},
            {"color13", "bright_magenta"}, {"color14", "bright_cyan"},
            {"color15", "bright_foreground"},
        };
        for (auto& [k, f] : kSemanticAnsi) Alias(k, f);
        for (auto& [k, l] : kLegacy)
            if (const std::string* v = Get(k)) entries[l] = *v;

        // mode: mode key, theme_type key, light.mode file, luminance, dark
        if (const std::string* v = Get("mode")) mode_ = *v;
        else if (const std::string* v = Get("theme_type")) mode_ = *v;
        else if (light_marker) mode_ = "light";
        else {
            const std::string* bg = Get("background");
            bool hex6 = bg && bg->size() == 7 && (*bg)[0] == '#';
            if (hex6) for (size_t i = 1; i < 7 && hex6; ++i)
                hex6 = std::isxdigit((unsigned char)(*bg)[i]) != 0;
            if (hex6) mode_ = Color("background").IsLight() ? "light" : "dark";
            else mode_ = "dark";
        }
        if (mode_ != "light") mode_ = "dark";
        entries["theme_type"] = mode_;
    }

    static Rgba Lit() { Rgba w; w.r = w.g = w.b = w.a = 1; return w; }

    // helper lives on Rgba via HexOut
};

// ---------------------------------------------------------------------------
// shell.toml tokens (flat sections; first-stop color flattening)
// ---------------------------------------------------------------------------

class Shell {
public:
    std::map<std::string, std::map<std::string, std::string>> sections;

    static Shell FromFile(const std::string& path) {
        Shell s;
        std::ifstream in(path);
        std::string line, current;
        while (std::getline(in, line)) {
            size_t a = line.find_first_not_of(" \t");
            if (a == std::string::npos) continue;
            std::string t = line.substr(a);
            t.erase(std::find_if(t.rbegin(), t.rend(),
                                 [](char c) { return !std::isspace((unsigned char)c); }).base(),
                    t.end());
            if (t.empty() || t[0] == '#') continue;
            if (t.front() == '[' && t.back() == ']') { current = t.substr(1, t.size() - 2); continue; }
            size_t eq = t.find('=');
            if (eq == std::string::npos) continue;
            std::string key = t.substr(0, eq);
            std::string value = t.substr(eq + 1);
            Trim(key);
            Trim(value);
            if (!value.empty() && value.front() == '"') {
                size_t e = value.find('"', 1);
                value = value.substr(1, e == std::string::npos ? std::string::npos : e - 1);
            } else {
                size_t h = value.find('#');
                if (h != std::string::npos) value = value.substr(0, h);
                Trim(value);
            }
            s.sections[current][key] = value;
        }
        return s;
    }

    std::string Str(const std::string& section, const std::string& key) const {
        auto it = sections.find(section);
        if (it == sections.end()) return "";
        auto jt = it->second.find(key);
        if (jt == it->second.end()) return "";
        std::string v = jt->second;
        // resolve one level of `section.key` references
        size_t dot = v.find('.');
        if (dot != std::string::npos && !v.empty() && v[0] != '#' &&
            !std::isdigit((unsigned char)v[0])) {
            std::string target = Str(v.substr(0, dot), v.substr(dot + 1));
            if (!target.empty()) return target;
        }
        return v;
    }

    float Num(const std::string& section, const std::string& key, float fallback) const {
        std::string v = Str(section, key);
        return v.empty() ? fallback : (float)std::atof(v.c_str());
    }

    Rgba Col(const std::string& section, const std::string& key) const {
        std::string v = Str(section, key);
        std::istringstream parts(v);
        std::string tok;
        while (parts >> tok) {
            if (!tok.empty() && tok[0] == '#') return Rgba::Hex(tok);
            if (tok.rfind("rgb", 0) == 0) {
                // rgba(r,g,b) form
                size_t l = tok.find('('), rr = tok.find(')');
                if (l != std::string::npos && rr != std::string::npos) {
                    std::string nums = tok.substr(l + 1, rr - l - 1);
                    for (char& c : nums) if (c == ',') c = ' ';
                    std::istringstream ss(nums);
                    int r = 0, g = 0, b = 0; float af = 1;
                    ss >> r >> g >> b >> af;
                    return {r / 255.f, g / 255.f, b / 255.f, af};
                }
            }
        }
        return {};
    }

private:
    static void Trim(std::string& s) {
        size_t a = s.find_first_not_of(" \t\r\n");
        size_t b = s.find_last_not_of(" \t\r\n");
        if (a == std::string::npos) { s.clear(); return; }
        s = s.substr(a, b - a + 1);
    }
};

// ---------------------------------------------------------------------------
// Theme: state paths, load, poll-for-changes
// ---------------------------------------------------------------------------

class Theme;
void ApplyStyle(ImGuiStyle& style, const Theme& theme);
void AutoApply(ImGuiStyle& style);

class Theme {
public:
    Palette palette;
    Shell shell;
    std::string name;
    std::string stateDir;   // .../omarchy/current
    std::string fontFamily; // best-effort, "" if unavailable
    std::string fontPath;
    std::string backgroundPath;
    bool loaded = false;

    static std::string CurrentStateDir() {
        const char* xdg = std::getenv("XDG_STATE_HOME");
        std::string base = (xdg && *xdg) ? xdg : "";
        if (base.empty()) {
            const char* home = std::getenv("HOME");
            base = home ? std::string(home) + "/.local/state" : "/tmp";
        }
        return base + "/omarchy/current";
    }

    Theme() : stateDir(CurrentStateDir()) { Load(); } // NOLINT: ergonomics first

    bool Load() {
        std::string themeDir = stateDir + "/theme";
        std::ifstream colors(themeDir + "/colors.toml");
        if (!colors) { loaded = false; return false; }
        bool lightMarker = FileExists(themeDir + "/light.mode");
        palette = Palette::FromFile(themeDir + "/colors.toml", lightMarker);
        shell = Shell::FromFile(themeDir + "/shell.toml");
        name = ReadTrimmed(stateDir + "/theme.name");
        backgroundPath = ReadLink(stateDir + "/background");
        ResolveFont();
        signature_ = Capture();
        loaded = true;
        return true;
    }

    // Poll tier: cheap (3 reads + a stat). True once per desktop switch.
    bool Changed() const {
        std::ifstream probe(stateDir + "/theme/colors.toml");
        if (!probe) return false; // swap in flight
        return Capture() != signature_;
    }

    bool ReloadIfChanged() {
        if (!Changed()) return false;
        return Load();
    }

    Rgba Accent() const { return palette.Color("accent"); }
    Rgba Background() const { return palette.Color("background"); }
    Rgba Foreground() const { return palette.Color("foreground"); }
    bool Light() const { return palette.IsLight(); }

private:
    std::string signature_;

    std::string Capture() const {
        return ReadAll(stateDir + "/theme.name") + "\x1f" +
               ReadAll(stateDir + "/theme/colors.toml") + "\x1f" +
               ReadAll(stateDir + "/theme/shell.toml") + "\x1f" +
               ReadLink(stateDir + "/background");
    }

    static std::string ReadAll(const std::string& path) {
        std::ifstream in(path, std::ios::binary);
        if (!in) return "";
        std::ostringstream ss;
        ss << in.rdbuf();
        return ss.str();
    }

    static std::string ReadTrimmed(const std::string& path) {
        std::string s = ReadAll(path);
        size_t a = s.find_first_not_of(" \t\r\n");
        size_t b = s.find_last_not_of(" \t\r\n");
        return a == std::string::npos ? "" : s.substr(a, b - a + 1);
    }

    static bool FileExists(const std::string& path) {
        std::ifstream in(path);
        return (bool)in;
    }

    static std::string ReadLink(const std::string& path);

    void ResolveFont() {
        fontFamily.clear();
        fontPath.clear();
        FILE* pipe = popen("fc-match monospace -f '%{family}\\n%{file}\\n' 2>/dev/null", "r");
        if (!pipe) return;
        char buf[512];
        std::string out;
        while (fgets(buf, sizeof buf, pipe)) out += buf;
        pclose(pipe);
        size_t nl = out.find('\n');
        if (nl == std::string::npos) return;
        std::string family = out.substr(0, nl);
        size_t comma = family.find(',');
        fontFamily = comma == std::string::npos ? family : family.substr(0, comma);
        std::string file = out.substr(nl + 1);
        size_t b = file.find_last_not_of("\r\n");
        if (b != std::string::npos) file = file.substr(0, b + 1);
        if (!file.empty() && FileExists(file)) fontPath = file;
    }
};

// ---------------------------------------------------------------------------
// ApplyStyle — "mapping v1" (see crates/omarchy-theme/src/imgui_recipe.rs)
// ---------------------------------------------------------------------------

inline void ApplyStyle(ImGuiStyle& style, const Theme& theme) {
    const Palette& p = theme.palette;
    const Shell& sh = theme.shell;

    Rgba bg = p.Color("background"), dbg = p.Color("dark_background");
    Rgba ddbg = p.Color("darker_background"), lbg = p.Color("lighter_background");
    Rgba sel = p.Color("selection"), fg = p.Color("foreground");
    Rgba dfg = p.Color("dark_foreground"), bfg = p.Color("bright_foreground");
    Rgba muted = p.Color("muted"), accent = p.Color("accent");
    Rgba border = sh.Str("hyprland", "active-border").empty()
                     ? accent
                     : sh.Col("hyprland", "active-border");

    float hoverFill = sh.Num("controls", "hover-cursor-fill-alpha", 0.08f);
    float pressedFill = sh.Num("controls", "pressed-fill-alpha", 0.22f);
    float fontBase = sh.Num("font", "base-size", 12.0f);
    float spacingScale = sh.Num("spacing", "scale", 1.0f);
    float windowAlpha = sh.Num("menu", "background-alpha", 0.95f);
    float popupAlpha = sh.Num("popups", "background-alpha", 1.0f);
    if (windowAlpha <= 0.0f) windowAlpha = 0.95f;
    if (popupAlpha <= 0.0f) popupAlpha = 1.0f;

    Rgba frame = bg.Mix(lbg, 0.55f);
    Rgba frameHover = frame.Mix(fg, std::max(hoverFill, 0.08f));
    Rgba frameActive = frame.Mix(accent, std::max(pressedFill, 0.15f));
    Rgba button = frame.Mix(accent, 0.10f);
    Rgba buttonHover = button.Mix(accent, 0.43f);
    Rgba buttonActive = button.Mix(accent, 0.55f);

    // Text
    style.Colors[ImGuiCol_Text] = fg;
    style.Colors[ImGuiCol_TextDisabled] = dfg;
    // Backgrounds
    style.Colors[ImGuiCol_WindowBg] = bg.Alpha(windowAlpha);
    style.Colors[ImGuiCol_ChildBg] = ImVec4(0, 0, 0, 0);
    style.Colors[ImGuiCol_PopupBg] = bg.Alpha(popupAlpha);
    style.Colors[ImGuiCol_Border] = muted;
    style.Colors[ImGuiCol_BorderShadow] = ImVec4(0, 0, 0, 0);
    // Frames
    style.Colors[ImGuiCol_FrameBg] = frame;
    style.Colors[ImGuiCol_FrameBgHovered] = frameHover;
    style.Colors[ImGuiCol_FrameBgActive] = frameActive;
    // Title bar
    style.Colors[ImGuiCol_TitleBg] = dbg;
    style.Colors[ImGuiCol_TitleBgActive] = bg;
    style.Colors[ImGuiCol_TitleBgCollapsed] = dbg.Alpha(0.6f);
    style.Colors[ImGuiCol_MenuBarBg] = ddbg;
    // Scrollbar
    style.Colors[ImGuiCol_ScrollbarBg] = bg.Alpha(0.0f);
    style.Colors[ImGuiCol_ScrollbarGrab] = muted;
    style.Colors[ImGuiCol_ScrollbarGrabHovered] = dfg;
    style.Colors[ImGuiCol_ScrollbarGrabActive] = accent;
    // Checks / sliders
    style.Colors[ImGuiCol_CheckMark] = accent;
    style.Colors[ImGuiCol_SliderGrab] = accent.Mix(frame, 0.25f);
    style.Colors[ImGuiCol_SliderGrabActive] = bfg;
    // Buttons
    style.Colors[ImGuiCol_Button] = button;
    style.Colors[ImGuiCol_ButtonHovered] = buttonHover;
    style.Colors[ImGuiCol_ButtonActive] = buttonActive;
    // Headers
    style.Colors[ImGuiCol_Header] = bg.Mix(accent, 0.12f);
    style.Colors[ImGuiCol_HeaderHovered] = bg.Mix(accent, 0.25f);
    style.Colors[ImGuiCol_HeaderActive] = bg.Mix(accent, 0.40f);
    // Separators
    style.Colors[ImGuiCol_Separator] = muted;
    style.Colors[ImGuiCol_SeparatorHovered] = accent;
    style.Colors[ImGuiCol_SeparatorActive] = border;
    // Resize grips
    style.Colors[ImGuiCol_ResizeGrip] = muted.Alpha(0.35f);
    style.Colors[ImGuiCol_ResizeGripHovered] = accent;
    style.Colors[ImGuiCol_ResizeGripActive] = border;
    // Tabs
    style.Colors[ImGuiCol_Tab] = ddbg.Mix(fg, 0.06f);
    style.Colors[ImGuiCol_TabHovered] = ddbg.Mix(accent, 0.22f);
#if IMGUI_VERSION_NUM >= 19100
    style.Colors[ImGuiCol_TabSelected] = sel.Mix(fg, 0.04f);
    style.Colors[ImGuiCol_TabSelectedOverline] = accent;
    style.Colors[ImGuiCol_TabDimmed] = ddbg.Alpha(0.6f);
    style.Colors[ImGuiCol_TabDimmedSelected] = sel;
    style.Colors[ImGuiCol_TabDimmedSelectedOverline] = accent;
#else
    style.Colors[ImGuiCol_TabActive] = sel.Mix(fg, 0.04f);
    style.Colors[ImGuiCol_TabUnfocused] = ddbg.Alpha(0.6f);
    style.Colors[ImGuiCol_TabUnfocusedActive] = sel;
#endif
    // Plots
    style.Colors[ImGuiCol_PlotLines] = accent;
    style.Colors[ImGuiCol_PlotLinesHovered] = p.Color("orange");
    style.Colors[ImGuiCol_PlotHistogram] = p.Color("green");
    style.Colors[ImGuiCol_PlotHistogramHovered] = p.Color("bright_green");
    // Tables
    style.Colors[ImGuiCol_TableHeaderBg] = bg.Mix(lbg, 0.5f);
    style.Colors[ImGuiCol_TableBorderStrong] = muted;
    style.Colors[ImGuiCol_TableBorderLight] = muted.Alpha(0.5f);
    style.Colors[ImGuiCol_TableRowBg] = ImVec4(0, 0, 0, 0);
    style.Colors[ImGuiCol_TableRowBgAlt] = fg.Alpha(0.03f);
    // Drag and text
    style.Colors[ImGuiCol_TextSelectedBg] = sel;
    style.Colors[ImGuiCol_DragDropTarget] = border;
    // Nav / modals
    style.Colors[ImGuiCol_NavHighlight] = border;
    style.Colors[ImGuiCol_NavWindowingHighlight] = bfg.Alpha(0.7f);
    style.Colors[ImGuiCol_NavWindowingDimBg] = ddbg.Alpha(0.2f);
    style.Colors[ImGuiCol_ModalWindowDimBg] = ddbg.Alpha(0.6f);
#ifdef IMGUI_HAS_DOCK
    style.Colors[ImGuiCol_DockingPreview] = accent.Alpha(0.4f);
    style.Colors[ImGuiCol_DockingEmptyBg] = ddbg;
#endif

    // Shape: compositor owns window corners; frames follow the shell's feel.
    float round = (float)(int)(fontBase * 0.16f + 0.5f);
    style.FrameRounding = round;
    style.GrabRounding = round;
    style.TabRounding = round;
    style.PopupRounding = (float)(int)(fontBase * 0.20f + 0.5f);
    style.WindowRounding = 0.0f;
    style.ChildRounding = 0.0f;
    style.ScrollbarRounding = (float)(int)(fontBase * 0.45f + 0.5f);
    style.WindowBorderSize = 0.0f;
    style.FrameBorderSize = 1.0f;
    style.TabBorderSize = 0.0f;
    float sc = std::max(spacingScale, 0.5f);
    style.ItemSpacing = ImVec2((float)(int)(6.0f * sc), (float)(int)(4.0f * sc));
    style.ItemInnerSpacing = ImVec2((float)(int)(4.0f * sc), (float)(int)(4.0f * sc));
    style.FramePadding = ImVec2(8.0f * sc, (float)(int)(fontBase * 0.25f + 0.5f));
    style.WindowPadding = ImVec2(10.0f * sc, 8.0f * sc);
    style.ScrollbarSize = (float)(int)(fontBase * 0.9f + 0.5f);
    style.GrabMinSize = (float)(int)(fontBase * 0.6f + 0.5f);
}

// Auto tier: one call per frame; loads once, reloads + reapplies on desktop
// theme changes. Keeps its own static Theme instance.
inline void AutoApply(ImGuiStyle& style) {
    static Theme theme;
    static bool applied = false;
    if (!theme.loaded) {
        theme.Load();
        if (!theme.loaded) return; // no theme staged yet; try again next frame
    }
    if (!applied || theme.ReloadIfChanged()) {
        ApplyStyle(style, theme);
        applied = true;
    }
}

// out-of-line bits needing sys headers kept at the end so the header stays
// include-order-insensitive
} // namespace omarchy

#include <unistd.h>
inline std::string omarchy::Theme::ReadLink(const std::string& path) {
    char buf[4096];
    ssize_t n = ::readlink(path.c_str(), buf, sizeof buf - 1);
    if (n <= 0) return "";
    buf[n] = 0;
    std::string target(buf);
    if (!target.empty() && target[0] != '/' && path.find('/') != std::string::npos) {
        std::string dir = path.substr(0, path.rfind('/') + 1);
        target = dir + target;
    }
    return target;
}

#endif // OMARCHY_IMGUI_HPP
