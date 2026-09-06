//! Dear ImGui (imgui-rs) adapter for omarchy-theme — applies the normative
//! "mapping v1" recipe (`omarchy_theme::imgui_recipe`) to an `imgui::Style`.
//!
//! ```no_run
//! use omarchy_theme::Theme;
//!
//! let mut ctx = imgui::Context::create();
//! let mut theme = Theme::current()?;
//! omarchy_theme_imgui::apply(ctx.style_mut(), &theme);
//!
//! // per frame (poll tier):
//! if theme.changed() {
//!     theme.reload()?;
//!     omarchy_theme_imgui::apply(ctx.style_mut(), &theme);
//! }
//! # Ok::<(), omarchy_theme::Error>(())
//! ```

use imgui::{Context, Style, StyleColor};

use omarchy_theme::{imgui_recipe, Theme};

/// Apply the recipe's colors and scalars to a style from a loaded theme.
pub fn apply(style: &mut Style, theme: &Theme) {
    let manifest = omarchy_theme::export::Manifest::from_theme(theme);
    for (name, rgba) in imgui_recipe::mapping(&manifest) {
        if let Some(color) = style_color(name) {
            style[color] = rgba;
        }
    }
    for (name, v) in imgui_recipe::style_scalars(&manifest) {
        match name {
            "FrameRounding" => style.frame_rounding = v,
            "GrabRounding" => style.grab_rounding = v,
            "TabRounding" => style.tab_rounding = v,
            "PopupRounding" => style.popup_rounding = v,
            "WindowRounding" => style.window_rounding = v,
            "ChildRounding" => style.child_rounding = v,
            "ScrollbarRounding" => style.scrollbar_rounding = v,
            "WindowBorderSize" => style.window_border_size = v,
            "FrameBorderSize" => style.frame_border_size = v,
            "TabBorderSize" => style.tab_border_size = v,
            "ItemSpacingX" => style.item_spacing[0] = v,
            "ItemSpacingY" => style.item_spacing[1] = v,
            "ItemInnerSpacingX" => style.item_inner_spacing[0] = v,
            "ItemInnerSpacingY" => style.item_inner_spacing[1] = v,
            "FramePaddingX" => style.frame_padding[0] = v,
            "FramePaddingY" => style.frame_padding[1] = v,
            "WindowPaddingX" => style.window_padding[0] = v,
            "WindowPaddingY" => style.window_padding[1] = v,
            "ScrollbarSize" => style.scrollbar_size = v,
            "GrabMinSize" => style.grab_min_size = v,
            _ => {}
        }
    }
}

/// Convenience: load the live theme and apply it to a context's style.
/// Returns false when no Omarchy theme is staged (caller keeps its own
/// default styling).
pub fn apply_current(ctx: &mut Context) -> bool {
    match Theme::current() {
        Ok(theme) => {
            apply(ctx.style_mut(), &theme);
            true
        }
        Err(_) => false,
    }
}

/// Map a recipe member name (`ImGuiCol_X`) to imgui-rs's `StyleColor`,
/// tolerating enum renames across imgui versions (unknown names are
/// silently skipped, so the recipe stays forward-compatible).
fn style_color(name: &str) -> Option<StyleColor> {
    Some(match name {
        "Text" => StyleColor::Text,
        "TextDisabled" => StyleColor::TextDisabled,
        "WindowBg" => StyleColor::WindowBg,
        "ChildBg" => StyleColor::ChildBg,
        "PopupBg" => StyleColor::PopupBg,
        "Border" => StyleColor::Border,
        "BorderShadow" => StyleColor::BorderShadow,
        "FrameBg" => StyleColor::FrameBg,
        "FrameBgHovered" => StyleColor::FrameBgHovered,
        "FrameBgActive" => StyleColor::FrameBgActive,
        "TitleBg" => StyleColor::TitleBg,
        "TitleBgActive" => StyleColor::TitleBgActive,
        "TitleBgCollapsed" => StyleColor::TitleBgCollapsed,
        "MenuBarBg" => StyleColor::MenuBarBg,
        "ScrollbarBg" => StyleColor::ScrollbarBg,
        "ScrollbarGrab" => StyleColor::ScrollbarGrab,
        "ScrollbarGrabHovered" => StyleColor::ScrollbarGrabHovered,
        "ScrollbarGrabActive" => StyleColor::ScrollbarGrabActive,
        "CheckMark" => StyleColor::CheckMark,
        "SliderGrab" => StyleColor::SliderGrab,
        "SliderGrabActive" => StyleColor::SliderGrabActive,
        "Button" => StyleColor::Button,
        "ButtonHovered" => StyleColor::ButtonHovered,
        "ButtonActive" => StyleColor::ButtonActive,
        "Header" => StyleColor::Header,
        "HeaderHovered" => StyleColor::HeaderHovered,
        "HeaderActive" => StyleColor::HeaderActive,
        "Separator" => StyleColor::Separator,
        "SeparatorHovered" => StyleColor::SeparatorHovered,
        "SeparatorActive" => StyleColor::SeparatorActive,
        "ResizeGrip" => StyleColor::ResizeGrip,
        "ResizeGripHovered" => StyleColor::ResizeGripHovered,
        "ResizeGripActive" => StyleColor::ResizeGripActive,
        "Tab" => StyleColor::Tab,
        "TabHovered" => StyleColor::TabHovered,
        // imgui 1.90 calls the selected tab "TabActive" (renamed
        // TabSelected in 1.91); the recipe emits the new name.
        "TabSelected" => StyleColor::TabActive,
        "TabUnfocused" => StyleColor::TabUnfocused,
        "TabUnfocusedActive" => StyleColor::TabUnfocusedActive,
        "PlotLines" => StyleColor::PlotLines,
        "PlotLinesHovered" => StyleColor::PlotLinesHovered,
        "PlotHistogram" => StyleColor::PlotHistogram,
        "PlotHistogramHovered" => StyleColor::PlotHistogramHovered,
        "TableHeaderBg" => StyleColor::TableHeaderBg,
        "TableBorderStrong" => StyleColor::TableBorderStrong,
        "TableBorderLight" => StyleColor::TableBorderLight,
        "TableRowBg" => StyleColor::TableRowBg,
        "TableRowBgAlt" => StyleColor::TableRowBgAlt,
        "TextSelectedBg" => StyleColor::TextSelectedBg,
        "DragDropTarget" => StyleColor::DragDropTarget,
        "NavHighlight" => StyleColor::NavHighlight,
        "NavWindowingHighlight" => StyleColor::NavWindowingHighlight,
        "NavWindowingDimBg" => StyleColor::NavWindowingDimBg,
        "ModalWindowDimBg" => StyleColor::ModalWindowDimBg,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn applies_live_theme_without_panicking() {
        let Ok(theme) = Theme::current() else { return };
        let mut ctx = Context::create();
        apply(ctx.style_mut(), &theme);
        let style = ctx.style_mut();
        // recipe spot-checks against the palette
        let want = theme.palette.color("foreground").unwrap().to_f32_array();
        assert_eq!(style[StyleColor::Text], want);
        assert_eq!(style.window_rounding, 0.0);
        assert_eq!(style.window_border_size, 0.0);
    }

    #[test]
    fn applies_synthetic_theme() {
        // covers the no-live-desktop path
        let raw = "mode = \"dark\"\nbackground = #1e1e2e\nforeground = #cdd6f4\naccent = #89b4fa\nselection = #45475a\nmuted = #585b70\nred = #f38ba8\ngreen = #a6e3a1\nyellow = #f9e2af\nblue = #89b4fa\nmagenta = #f5c2e7\ncyan = #94e2d5\n";
        let palette = omarchy_theme::Palette::resolve(raw, false);
        let theme = Theme::from_palette(palette);
        let want_sel = theme.palette.color("selection").unwrap().to_f32_array();
        let mut ctx = Context::create();
        apply(ctx.style_mut(), &theme);
        assert_eq!(ctx.style_mut()[StyleColor::TextSelectedBg], want_sel);
    }
}
