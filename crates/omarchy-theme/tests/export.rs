//! Export format tests: deterministic synthetic theme, golden-checked
//! renderings. Pins the wire formats bindings/docs rely on.

use omarchy_theme::export::{self, Manifest, StyleManifest};
use omarchy_theme::Rgba;

fn synthetic_manifest() -> Manifest {
    let palette = omarchy_theme::Palette::resolve(
        "mode = \"dark\"\naccent = #89b4fa\nbackground = #1e1e2e\ndark_background = #181825\ndarker_background = #11111b\nlighter_background = #313244\nselection = #45475a\nmuted = #585b70\nforeground = #cdd6f4\ndark_foreground = #6c7086\nlight_foreground = #bac2de\nbright_foreground = #cdd6f4\nred = #f38ba8\nyellow = #f9e2af\norange = #fab387\ngreen = #a6e3a1\ncyan = #94e2d5\nblue = #89b4fa\nmagenta = #f5c2e7\nbrown = #7b5b55\n",
        false,
    );
    Manifest {
        theme: "goldentheme".into(),
        mode: palette.mode().as_str(),
        palette: export::resolved_palette(&palette),
        style: StyleManifest {
            normal_fill_alpha: 0.04,
            normal_border_alpha: 0.4,
            normal_border_width: 1.0,
            hover_fill_alpha: 0.08,
            hover_border_alpha: 0.25,
            focus_fill_alpha: 0.08,
            selected_fill_alpha: 0.18,
            pressed_fill_alpha: 0.22,
            selection_fill_alpha: 0.35,
            spacing_scale: 1.0,
            font_base_size: 12.0,
            active_border: Rgba::parse("#89b4fa"),
            menu_alpha: Some(0.95),
            popup_alpha: Some(1.0),
        },
        font_family: "Test Mono NF".into(),
        font_path: "/usr/share/fonts/test.ttf".into(),
        background_path: "/wall/scene.png".into(),
    }
}

#[test]
fn json_manifest_golden_fields() {
    let m = synthetic_manifest();
    let json: serde_json::Value = serde_json::from_str(&export::to_json(&m)).unwrap();
    assert_eq!(json["theme"], "goldentheme");
    assert_eq!(json["mode"], "dark");
    assert_eq!(json["palette"]["accent"], "#89b4fa");
    assert_eq!(json["palette"]["cursor"], "#cdd6f4");
    assert_eq!(json["palette"]["color1"], "#f38ba8");
    assert_eq!(json["style"]["hover-fill-alpha"], 0.08);
    assert_eq!(json["style"]["active-border"], "#89b4fa");
    assert_eq!(json["font"]["family"], "Test Mono NF");
    assert_eq!(json["background"], "/wall/scene.png");
}

#[test]
fn css_golden() {
    let css = export::to_css(&synthetic_manifest());
    assert!(css.starts_with(":root {"));
    assert!(css.contains("--omarchy-accent: #89b4fa;"));
    assert!(css.contains("--omarchy-cursor: #cdd6f4;"));
    assert!(css.contains("--omarchy-hover-fill-alpha: 0.08;"));
    assert!(css.contains("font-family: \"Test Mono NF\""));
}

#[test]
fn toml_golden() {
    let toml = export::to_toml(&synthetic_manifest());
    assert!(toml.contains("[meta]\ntheme = \"goldentheme\"\nmode = \"dark\""));
    assert!(toml.contains("accent = \"#89b4fa\"\n")); // quoted: bare # is a TOML comment
    assert!(toml.contains("[style]\nnormal-fill-alpha = 0.04"));
    assert!(toml.contains("active-border = \"#89b4fa\""));
}

#[test]
fn c_header_golden() {
    let h = export::to_c_header(&synthetic_manifest());
    assert!(h.contains("#define OMARCHY_THEME_NAME \"goldentheme\""));
    assert!(h.contains("#define OMARCHY_THEME_MODE_DARK"));
    assert!(h.contains("#define OMC_ACCENT 0x89B4FAFFu /* #89b4fa */"));
    assert!(h.contains("#define OMS_HOVER_FILL_ALPHA 0.08f"));
    assert!(h.contains("#endif"));
}

#[test]
fn imgui_hpp_names_and_ranges() {
    let h = export::to_imgui_hpp(&synthetic_manifest());
    assert!(h.contains("#include \"imgui.h\""));
    assert!(h.contains("style.Colors[ImGuiCol_Text] ="));
    assert!(h.contains("style.Colors[ImGuiCol_TextSelectedBg] ="));
    assert!(h.contains("style.Colors[ImGuiCol_Tab] ="));
    // alpha-0 entries: transparent ChildBg stays exactly transparent
    assert!(h.contains("style.Colors[ImGuiCol_ChildBg] = ImVec4(0.000000f, 0.000000f, 0.000000f, 0.000000f);"));
    // scalars
    assert!(h.contains("style.WindowRounding = 0.0000f;"));
    for line in h.lines().filter(|l| l.contains("ImVec4(")) {
        // all components within [0,1]
        let nums: Vec<f32> = line
            .split("ImVec4(")
            .nth(1)
            .unwrap()
            .trim_end_matches(");")
            .split(',')
            .map(|p| p.trim_end_matches('f').trim().parse().unwrap())
            .collect();
        assert_eq!(nums.len(), 4, "line: {line}");
        for n in nums {
            assert!((0.0..=1.0).contains(&n), "out of range in {line}");
        }
    }
}

#[test]
fn light_theme_uses_same_recipe_without_special_case() {
    let mut m = synthetic_manifest();
    m.palette.insert("mode".into(), "light".into());
    m.palette.insert("theme_type".into(), "light".into());
    m.mode = "light";
    let h = export::to_imgui_hpp(&m);
    assert!(h.contains("// Theme: goldentheme (light)"));
    assert!(h.contains("style.Colors[ImGuiCol_Text] ="));
}
