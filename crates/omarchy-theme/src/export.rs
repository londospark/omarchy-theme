//! Language-neutral export formats: everything the CLI emits is produced
//! here, so generated files and hand-written bindings agree.
//!
//! The Dear ImGui color recipe lives in [`imgui_mapping`] and is the single
//! normative description of "Omarchy -> ImGuiStyle" (the runtime C++ header
//! mirrors it; `docs/contract.md` §6 explains when each is appropriate).

use std::collections::BTreeMap;

use crate::color::Rgba;
use crate::palette::Palette;
use crate::style::Style;
use crate::Theme;

/// The serializable manifest (also the shape `export --format json` emits).
#[derive(Clone, Debug)]
pub struct Manifest {
    pub theme: String,
    pub mode: &'static str,
    pub palette: BTreeMap<String, String>,
    pub style: StyleManifest,
    pub font_family: String,
    pub font_path: String,
    pub background_path: String,
}

#[derive(Clone, Debug)]
pub struct StyleManifest {
    pub normal_fill_alpha: f32,
    pub normal_border_alpha: f32,
    pub normal_border_width: f32,
    pub hover_fill_alpha: f32,
    pub hover_border_alpha: f32,
    pub focus_fill_alpha: f32,
    pub selected_fill_alpha: f32,
    pub pressed_fill_alpha: f32,
    pub selection_fill_alpha: f32,
    pub spacing_scale: f32,
    pub font_base_size: f32,
    pub active_border: Option<Rgba>,
    /// `[menu] background-alpha` from shell.toml, when present.
    pub menu_alpha: Option<f32>,
    /// `[popups] background-alpha` from shell.toml, when present.
    pub popup_alpha: Option<f32>,
}

impl Manifest {
    pub fn from_theme(theme: &Theme) -> Manifest {
        Manifest {
            theme: theme.name.clone().unwrap_or_else(|| "unknown".into()),
            mode: theme.palette.mode().as_str(),
            palette: theme
                .palette
                .entries()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
            style: StyleManifest::from_style(&theme.style),
            font_family: theme
                .font
                .as_ref()
                .map(|f| f.family.clone())
                .unwrap_or_default(),
            font_path: theme
                .font
                .as_ref()
                .and_then(|f| f.file.as_ref())
                .map(|p| p.display().to_string())
                .unwrap_or_default(),
            background_path: theme
                .background
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_default(),
        }
    }

    pub fn palette_color(&self, key: &str) -> Option<Rgba> {
        self.palette.get(key).and_then(|v| Rgba::parse(v))
    }
}

impl StyleManifest {
    pub fn from_style(s: &Style) -> StyleManifest {
        StyleManifest {
            normal_fill_alpha: s.normal.fill_alpha,
            normal_border_alpha: s.normal.border_alpha,
            normal_border_width: s.normal.border_width,
            hover_fill_alpha: s.hover.fill_alpha,
            hover_border_alpha: s.hover.border_alpha,
            focus_fill_alpha: s.focus.fill_alpha,
            selected_fill_alpha: s.selected.fill_alpha,
            pressed_fill_alpha: s.pressed_fill_alpha,
            selection_fill_alpha: s.selection_fill_alpha,
            spacing_scale: s.spacing_scale,
            font_base_size: s.font_base_size,
            active_border: s.active_border,
            menu_alpha: s.menu.map(|m| m.background_alpha),
            popup_alpha: s.popup.map(|m| m.background_alpha),
        }
    }
}

/// Resolved palette as sorted raw strings — what `--format toml` and CSS
/// custom properties emit.
pub fn resolved_palette(palette: &Palette) -> BTreeMap<String, String> {
    palette
        .entries()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

// ---------------------------------------------------------------------------
// JSON
// ---------------------------------------------------------------------------

#[cfg(feature = "json")]
pub fn to_json(m: &Manifest) -> String {
    let mut map = serde_json::Map::new();
    map.insert("theme".into(), m.theme.clone().into());
    map.insert("mode".into(), m.mode.into());
    let mut colors = serde_json::Map::new();
    for (k, v) in &m.palette {
        colors.insert(k.clone(), v.clone().into());
    }
    map.insert("palette".into(), colors.into());
    let mut style = serde_json::Map::new();
    // f32 -> f64 roundtrip leaves noise (0.08 -> 0.0799999982...); six
    // decimals is plenty for alphas/scales and keeps output stable.
    let num = |v: f32| {
        serde_json::Number::from_f64(((v as f64) * 1e6).round() / 1e6)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null)
    };
    for (k, v) in [
        ("normal-fill-alpha", m.style.normal_fill_alpha),
        ("normal-border-alpha", m.style.normal_border_alpha),
        ("normal-border-width", m.style.normal_border_width),
        ("hover-fill-alpha", m.style.hover_fill_alpha),
        ("hover-border-alpha", m.style.hover_border_alpha),
        ("focus-fill-alpha", m.style.focus_fill_alpha),
        ("selected-fill-alpha", m.style.selected_fill_alpha),
        ("pressed-fill-alpha", m.style.pressed_fill_alpha),
        ("selection-fill-alpha", m.style.selection_fill_alpha),
        ("spacing-scale", m.style.spacing_scale),
        ("font-base-size", m.style.font_base_size),
    ] {
        style.insert(k.into(), num(v));
    }
    if let Some(ab) = m.style.active_border {
        style.insert("active-border".into(), ab.to_hex().into());
    }
    map.insert("style".into(), style.into());
    map.insert(
        "font".into(),
        serde_json::json!({ "family": m.font_family, "path": m.font_path }),
    );
    map.insert("background".into(), m.background_path.clone().into());
    let root = serde_json::Value::Object(map);
    serde_json::to_string_pretty(&root).unwrap() + "\n"
}

// ---------------------------------------------------------------------------
// CSS custom properties (Tauri/webview apps)
// ---------------------------------------------------------------------------

pub fn to_css(m: &Manifest) -> String {
    let mut out = String::new();
    out.push_str(":root {\n");
    out.push_str(&format!("  --omarchy-theme: {};\n", m.theme));
    out.push_str(&format!("  --omarchy-mode: {};\n", m.mode));
    for (k, v) in &m.palette {
        out.push_str(&format!("  --omarchy-{k}: {v};\n"));
    }
    let s = &m.style;
    out.push_str(&format!(
        "  --omarchy-normal-fill-alpha: {};\n",
        s.normal_fill_alpha
    ));
    out.push_str(&format!(
        "  --omarchy-hover-fill-alpha: {};\n",
        s.hover_fill_alpha
    ));
    out.push_str(&format!(
        "  --omarchy-focus-fill-alpha: {};\n",
        s.focus_fill_alpha
    ));
    out.push_str(&format!(
        "  --omarchy-selected-fill-alpha: {};\n",
        s.selected_fill_alpha
    ));
    out.push_str(&format!(
        "  --omarchy-pressed-fill-alpha: {};\n",
        s.pressed_fill_alpha
    ));
    out.push_str(&format!(
        "  --omarchy-spacing-scale: {};\n",
        s.spacing_scale
    ));
    out.push_str(&format!(
        "  --omarchy-font-base-size: {}px;\n",
        s.font_base_size
    ));
    out.push_str("}\n");
    if !m.font_family.is_empty() {
        out.push_str(&format!(
            "\nbody {{ font-family: \"{}\", ui-monospace, monospace; }}\n",
            m.font_family
        ));
    }
    out
}

// ---------------------------------------------------------------------------
// Flat TOML (config-file consumers)
// ---------------------------------------------------------------------------

pub fn to_toml(m: &Manifest) -> String {
    let mut out = String::new();
    out.push_str(&format!("# omarchy-theme export: {}\n\n[meta]\n", m.theme));
    out.push_str(&format!("theme = \"{}\"\nmode = \"{}\"\n", m.theme, m.mode));
    if !m.font_family.is_empty() {
        out.push_str(&format!("font-family = \"{}\"\n", m.font_family));
    }
    if !m.font_path.is_empty() {
        out.push_str(&format!("font-path = \"{}\"\n", m.font_path));
    }
    if !m.background_path.is_empty() {
        out.push_str(&format!("background = \"{}\"\n", m.background_path));
    }
    out.push_str("\n[palette]\n");
    for (k, v) in &m.palette {
        let value = if needs_quotes(v) {
            format!("\"{v}\"")
        } else {
            v.clone()
        };
        out.push_str(&format!("{k} = {value}\n"));
    }
    out.push_str("\n[style]\n");
    let s = &m.style;
    out.push_str(&format!("normal-fill-alpha = {}\n", s.normal_fill_alpha));
    out.push_str(&format!(
        "normal-border-alpha = {}\n",
        s.normal_border_alpha
    ));
    out.push_str(&format!(
        "normal-border-width = {}\n",
        s.normal_border_width
    ));
    out.push_str(&format!("hover-fill-alpha = {}\n", s.hover_fill_alpha));
    out.push_str(&format!("hover-border-alpha = {}\n", s.hover_border_alpha));
    out.push_str(&format!("focus-fill-alpha = {}\n", s.focus_fill_alpha));
    out.push_str(&format!(
        "selected-fill-alpha = {}\n",
        s.selected_fill_alpha
    ));
    out.push_str(&format!("pressed-fill-alpha = {}\n", s.pressed_fill_alpha));
    out.push_str(&format!(
        "selection-fill-alpha = {}\n",
        s.selection_fill_alpha
    ));
    out.push_str(&format!("spacing-scale = {}\n", s.spacing_scale));
    out.push_str(&format!("font-base-size = {}\n", s.font_base_size));
    if let Some(ab) = s.active_border {
        out.push_str(&format!("active-border = \"{}\"\n", ab.to_hex()));
    }
    out
}

fn needs_quotes(value: &str) -> bool {
    // bare TOML values may only be numbers/booleans; `#...` would parse as
    // a comment there, so every color (and any string) is quoted.
    value.parse::<f64>().is_err()
}

// ---------------------------------------------------------------------------
// C header (#defines + POD tables) — for apps that want zero runtime deps
// ---------------------------------------------------------------------------

pub fn to_c_header(m: &Manifest) -> String {
    let mut out = String::new();
    let guard = format!(
        "OMARCHY_THEME_GENERATED_{}_H",
        m.theme.to_uppercase().replace('-', "_")
    );
    out.push_str(&format!("/* Generated by omarchy-theme export --format c-header\n from theme `{}`; regenerate on theme change (see hook/omarchy-theme-gen). */\n", m.theme));
    out.push_str(&format!("#ifndef {guard}\n#define {guard}\n\n"));
    out.push_str(&format!("#define OMARCHY_THEME_NAME \"{}\"\n", m.theme));
    out.push_str(&format!(
        "#define OMARCHY_THEME_MODE_{}\n",
        m.mode.to_uppercase()
    ));
    out.push_str("\n/* palette (0xRRGGBBAA) */\n");
    for (k, v) in &m.palette {
        if let Some(c) = Rgba::parse(v) {
            let ident = k.to_uppercase();
            out.push_str(&format!(
                "#define OMC_{ident} 0x{:08X}u /* {v} */\n",
                c.to_u32()
            ));
        }
    }
    out.push_str("\n/* shell style tokens */\n");
    let s = &m.style;
    out.push_str(&format!(
        "#define OMS_NORMAL_FILL_ALPHA {:?}f\n",
        s.normal_fill_alpha
    ));
    out.push_str(&format!(
        "#define OMS_HOVER_FILL_ALPHA {:?}f\n",
        s.hover_fill_alpha
    ));
    out.push_str(&format!(
        "#define OMS_SELECTED_FILL_ALPHA {:?}f\n",
        s.selected_fill_alpha
    ));
    out.push_str(&format!(
        "#define OMS_PRESSED_FILL_ALPHA {:?}f\n",
        s.pressed_fill_alpha
    ));
    out.push_str(&format!(
        "#define OMS_SPACING_SCALE {:?}f\n",
        s.spacing_scale
    ));
    out.push_str(&format!(
        "#define OMS_FONT_BASE_SIZE {:?}f\n",
        s.font_base_size
    ));
    out.push_str("\n#endif\n");
    out
}

// ---------------------------------------------------------------------------
// Dear ImGui — the normative color recipe, shared by codegen and runtime
// ---------------------------------------------------------------------------

/// One (ImGuiCol member name, RGBA float4) pair per style color the recipe
/// defines; members absent from the list are left untouched by adapters.
pub fn imgui_mapping(m: &Manifest) -> Vec<(&'static str, [f32; 4])> {
    use crate::imgui_recipe::mapping;
    mapping(m)
}

/// Non-color ImGuiStyle scalars from the recipe.
pub fn imgui_style_scalars(m: &Manifest) -> Vec<(&'static str, f32)> {
    use crate::imgui_recipe::style_scalars;
    style_scalars(m)
}

/// C++ header emitting a function that applies this theme's generated table
/// by ImGuiCol member name (robust across imgui versions; pairs with the
/// docking branch).
pub fn to_imgui_hpp(m: &Manifest) -> String {
    let mut out = String::new();
    out.push_str("// Generated by omarchy-theme export --format imgui.hpp\n");
    out.push_str(&format!("// Theme: {} ({})\n", m.theme, m.mode));
    out.push_str("#pragma once\n#include \"imgui.h\"\n#include <cmath>\n\n");
    out.push_str("namespace omarchy_generated {\ninline void ApplyStyle(ImGuiStyle& style) {\n");
    for (name, c) in imgui_mapping(m) {
        out.push_str(&format!(
            "    style.Colors[ImGuiCol_{name}] = ImVec4({:.6}f, {:.6}f, {:.6}f, {:.6}f);\n",
            c[0], c[1], c[2], c[3]
        ));
    }
    for (name, v) in imgui_style_scalars(m) {
        if name.contains('.') || name.contains('[') {
            // tuple members like WindowPadding.x
            out.push_str(&format!("    style.{name} = {v:.4}f;\n"));
        } else {
            out.push_str(&format!("    style.{name} = {v:.4}f;\n"));
        }
    }
    out.push_str("}\n} // namespace omarchy_generated\n");
    out
}
