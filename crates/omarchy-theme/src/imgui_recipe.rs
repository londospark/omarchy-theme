//! The normative Omarchy -> Dear ImGui style recipe ("mapping v1").
//!
//! Design rules, shared by the codegen export (`--format imgui.hpp`), the
//! runtime C++ header, and the Rust/egui/iced adapters:
//!
//! 1. *Opaque mixes over alphas.* Hyprland already composites window alpha;
//!    baking state changes as `mix(surface, layer, shell_alpha)` keeps widgets
//!    legible without double-transparency. Alphas appear only on window/popups.
//! 2. *Accent is the interaction hue; foreground is the chrome hue.* Buttons,
//!    sliders, checks, and headers tint with `accent`; frames and tabs use the
//!    background ramp and `foreground`/`muted` per shell.toml's control model.
//! 3. *State alphas come from shell.toml `[controls]`* when present, so apps
//!    track the desktop's hover/selected/pressed feel per theme.
//! 4. Light themes need no special-casing: the ramp direction is already
//!    flipped in the palette (darker stops are *more* prominent in light mode).

use crate::color::Rgba;
use crate::export::Manifest;

fn mix3(a: Rgba, b: Rgba, t: f64) -> Rgba {
    a.mix(b, t)
}

fn col(m: &Manifest, key: &str) -> Rgba {
    m.palette_color(key).unwrap_or(Rgba::BLACK)
}

fn f4(c: Rgba) -> [f32; 4] {
    c.to_f32_array()
}

/// `(ImGuiCol_ member, RGBA float4)` for every color the recipe sets.
pub fn mapping(m: &Manifest) -> Vec<(&'static str, [f32; 4])> {
    let bg = col(m, "background");
    let dbg = col(m, "dark_background");
    let ddbg = col(m, "darker_background");
    let lbg = col(m, "lighter_background");
    let sel = col(m, "selection");
    let fg = col(m, "foreground");
    let dfg = col(m, "dark_foreground");
    let bfg = col(m, "bright_foreground");
    let muted = col(m, "muted");
    let accent = col(m, "accent");
    let active_border = m.style.active_border.unwrap_or(accent);

    // shell state alphas (defaults match default/themed/shell.toml.tpl)
    let s = &m.style;

    let frame = mix3(bg, lbg, 0.55);
    let frame_hover = mix3(frame, fg, s.hover_fill_alpha.max(0.08) as f64);
    let frame_active = mix3(frame, accent, s.pressed_fill_alpha.max(0.15) as f64);

    let button = mix3(frame, accent, 0.10);
    let button_hover = mix3(button, accent, s.hover_fill_alpha.max(0.12) as f64 / 0.28);
    let button_active = mix3(button, accent, 0.55);

    vec![
        ("Text", f4(fg)),
        ("TextDisabled", f4(dfg)),
        ("WindowBg", f4(bg.with_alpha(window_alpha(m)))),
        ("ChildBg", [0.0, 0.0, 0.0, 0.0]),
        (
            "PopupBg",
            f4(col(m, "background").with_alpha(popup_alpha(m))),
        ),
        ("Border", f4(muted)),
        ("BorderShadow", [0.0, 0.0, 0.0, 0.0]),
        ("FrameBg", f4(frame)),
        ("FrameBgHovered", f4(frame_hover)),
        ("FrameBgActive", f4(frame_active)),
        ("TitleBg", f4(dbg)),
        ("TitleBgActive", f4(bg)),
        ("TitleBgCollapsed", f4(dbg.with_alpha(0.6))),
        ("MenuBarBg", f4(ddbg)),
        ("ScrollbarBg", f4(bg.with_alpha(0.0))),
        ("ScrollbarGrab", f4(muted)),
        ("ScrollbarGrabHovered", f4(dfg)),
        ("ScrollbarGrabActive", f4(accent)),
        ("CheckMark", f4(accent)),
        ("SliderGrab", f4(mix3(accent, frame, 0.25))),
        ("SliderGrabActive", f4(bfg)),
        ("Button", f4(button)),
        ("ButtonHovered", f4(button_hover)),
        ("ButtonActive", f4(button_active)),
        ("Header", f4(mix3(bg, accent, 0.12))),
        ("HeaderHovered", f4(mix3(bg, accent, 0.25))),
        ("HeaderActive", f4(mix3(bg, accent, 0.40))),
        ("Separator", f4(muted)),
        ("SeparatorHovered", f4(accent)),
        ("SeparatorActive", f4(active_border)),
        ("ResizeGrip", f4(muted.with_alpha(0.35))),
        ("ResizeGripHovered", f4(accent)),
        ("ResizeGripActive", f4(active_border)),
        ("Tab", f4(mix3(ddbg, fg, 0.06))),
        ("TabHovered", f4(mix3(ddbg, accent, 0.22))),
        ("TabSelected", f4(mix3(sel, fg, 0.04))),
        ("TabUnfocused", f4(ddbg.with_alpha(0.6))),
        ("TabUnfocusedActive", f4(sel)),
        ("PlotLines", f4(accent)),
        ("PlotLinesHovered", f4(col(m, "orange"))),
        ("PlotHistogram", f4(col(m, "green"))),
        ("PlotHistogramHovered", f4(col(m, "bright_green"))),
        ("TableHeaderBg", f4(mix3(bg, lbg, 0.5))),
        ("TableBorderStrong", f4(muted)),
        ("TableBorderLight", f4(muted.with_alpha(0.5))),
        ("TableRowBg", [0.0, 0.0, 0.0, 0.0]),
        ("TableRowBgAlt", f4(fg.with_alpha(0.03))),
        ("TextSelectedBg", f4(sel)),
        ("DragDropTarget", f4(active_border)),
        ("NavHighlight", f4(active_border)),
        ("NavWindowingHighlight", f4(bfg.with_alpha(0.7))),
        ("NavWindowingDimBg", f4(ddbg.with_alpha(0.2))),
        ("ModalWindowDimBg", f4(ddbg.with_alpha(0.6))),
        // docking branch (harmless if absent: adapters index by name)
        ("DockingPreview", f4(accent.with_alpha(0.4))),
        ("DockingEmptyBg", f4(ddbg)),
    ]
}

fn window_alpha(m: &Manifest) -> f32 {
    // Hyprland default inactive/active opacity vibe; popup/menu tokens from
    // shell.toml win when present.
    m.style.menu_alpha.unwrap_or(0.95)
}

fn popup_alpha(m: &Manifest) -> f32 {
    m.style.popup_alpha.unwrap_or(1.0)
}

/// ImGuiStyle scalar members the recipe pins. Names are Rust-side paths; the
/// adapters keep their own concrete assignment.
pub fn style_scalars(m: &Manifest) -> Vec<(&'static str, f32)> {
    let base = m.style.font_base_size.max(10.0);
    let scale = m.style.spacing_scale.max(0.5);
    vec![
        ("FrameRounding", (base * 0.16).round()),
        ("GrabRounding", (base * 0.16).round()),
        ("TabRounding", (base * 0.16).round()),
        ("PopupRounding", (base * 0.20).round()),
        ("WindowRounding", 0.0), // compositor draws the rounded corners
        ("ChildRounding", 0.0),
        ("ScrollbarRounding", (base * 0.45).round()),
        ("WindowBorderSize", 0.0), // compositor draws borders
        ("FrameBorderSize", 1.0),
        ("TabBorderSize", 0.0),
        ("ItemSpacingX", (6.0 * scale).round()),
        ("ItemSpacingY", (4.0 * scale).round()),
        ("ItemInnerSpacingX", (4.0 * scale).round()),
        ("ItemInnerSpacingY", (4.0 * scale).round()),
        ("FramePaddingX", (8.0 * scale).round()),
        ("FramePaddingY", (base * 0.25).round()),
        ("WindowPaddingX", (10.0 * scale).round()),
        ("WindowPaddingY", (8.0 * scale).round()),
        ("ScrollbarSize", (base * 0.9).round()),
        ("GrabMinSize", (base * 0.6).round()),
    ]
}
