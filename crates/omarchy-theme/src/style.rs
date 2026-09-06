//! `shell.toml` parsing: the Omarchy shell's semantic style tokens (control
//! states, surfaces, spacing, font scale) as a shared app-side style model.
//!
//! The file is flat TOML: `[section]` headers with `key = value` lines. This
//! parser understands the subset Omarchy generates (strings, numbers,
//! booleans) without pulling in a full TOML dependency, and resolves the two
//! indirection conventions the template engine uses:
//!
//! * references — `border = "hyprland.active-border"` names `section.key`
//!   in the same file;
//! * gradients — `active-border = "rgba(..) rgba(..) 45deg"`; consumers that
//!   cannot render them take the first stop (the same flattening the shell's
//!   `Color.<section>.border` singletons do).

use std::collections::BTreeMap;

use crate::color::Rgba;

/// A parsed shell.toml with resolved section references.
#[derive(Clone, Debug, Default)]
pub struct StyleTokens {
    /// section -> key -> raw string value (quotes removed, refs resolved).
    sections: BTreeMap<String, BTreeMap<String, String>>,
}

impl StyleTokens {
    /// Parse `shell.toml` content. Unknown/extra sections are preserved;
    /// malformed lines are skipped like the flat parser in the palette does.
    pub fn parse(raw: &str) -> StyleTokens {
        let mut sections: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
        let mut current = String::new();
        for line in raw.lines() {
            let line = line.trim_end();
            let t = line.trim_start();
            if t.is_empty() || t.starts_with('#') {
                continue;
            }
            if let Some(rest) = t.strip_prefix('[') {
                if let Some(name) = rest.strip_suffix(']') {
                    current = name.trim().to_string();
                    sections.entry(current.clone()).or_default();
                }
                continue;
            }
            let Some((key, value)) = t.split_once('=') else { continue };
            let key = key.trim().to_string();
            if key.is_empty() || !key.bytes().all(|c| c.is_ascii_alphanumeric() || matches!(c, b'_' | b'-' | b'.')) {
                continue;
            }
            let value = clean_value(value.trim());
            sections.entry(current.clone()).or_default().insert(key, value);
        }

        let mut s = StyleTokens { sections };
        s.resolve_references();
        s
    }

    fn resolve_references(&mut self) {
        // A bare `section.key` string value is a reference; iterate to a
        // fixed point so chained references settle (cycle-safe: a reference
        // that never settles keeps its original value).
        for _ in 0..4 {
            let mut changed_any = false;
            let snapshot = self.sections.clone();
            for entries in self.sections.values_mut() {
                for value in entries.values_mut() {
                    let Some((rsec, rkey)) = split_reference(value) else { continue };
                    let Some(target) = snapshot.get(rsec).and_then(|m| m.get(rkey)) else {
                        continue;
                    };
                    if target != value {
                        *value = target.clone();
                        changed_any = true;
                    }
                }
            }
            if !changed_any {
                break;
            }
        }
    }

    pub fn get(&self, section: &str, key: &str) -> Option<&str> {
        self.sections.get(section)?.get(key).map(String::as_str)
    }

    pub fn number(&self, section: &str, key: &str) -> Option<f64> {
        self.get(section, key)?.parse().ok()
    }

    pub fn bool(&self, section: &str, key: &str) -> Option<bool> {
        match self.get(section, key)? {
            "true" => Some(true),
            "false" => Some(false),
            _ => None,
        }
    }

    /// The first parseable color in a value: a flat color, the first stop of
    /// a gradient spec, or the resolved first stop of a reference.
    pub fn color(&self, section: &str, key: &str) -> Option<Rgba> {
        let value = self.get(section, key)?;
        for token in split_top_level_tokens(value) {
            if let Some(c) = Rgba::parse(token) {
                return Some(c);
            }
        }
        None
    }

    /// Section names present in the file.
    pub fn sections(&self) -> impl Iterator<Item = &str> {
        self.sections.keys().map(String::as_str)
    }

    /// Iterate a section's key/value pairs.
    pub fn iter_section(&self, section: &str) -> impl Iterator<Item = (&str, &str)> {
        self.sections
            .get(section)
            .into_iter()
            .flat_map(|m| m.iter().map(|(k, v)| (k.as_str(), v.as_str())))
    }

    pub fn is_empty(&self) -> bool {
        self.sections.is_empty()
    }

    pub fn raw(&self) -> &BTreeMap<String, BTreeMap<String, String>> {
        &self.sections
    }
}

/// `"hyprland.active-border"` -> Some(("hyprland", "active-border")); only
/// when such a section exists (so `1.0` and colors never misfire).
fn split_reference(value: &str) -> Option<(&str, &str)> {
    let dot = value.find('.')?;
    let head = &value[..dot];
    let tail = &value[dot + 1..];
    if head.is_empty() || tail.is_empty() || tail.contains('.') {
        return None;
    }
    if !head.bytes().all(|c| c.is_ascii_alphanumeric() || matches!(c, b'_' | b'-'))
        || !tail.bytes().all(|c| c.is_ascii_alphanumeric() || matches!(c, b'_' | b'-'))
    {
        return None;
    }
    // A dotted value that parses as something else (a number, a hex color)
    // is not a reference; presence of the target section is checked by the
    // caller's lookup.
    if value.contains(['#', '(']) || head.starts_with(|c: char| c.is_ascii_digit()) {
        return None;
    }
    Some((head, tail))
}

/// Split on whitespace outside parentheses: `"rgba(1 2 3) #aabbcc 45deg"`
/// yields two tokens, keeping gradient stops intact.
fn split_top_level_tokens(value: &str) -> Vec<&str> {
    let mut tokens = Vec::new();
    let mut depth = 0usize;
    let mut start = 0usize;
    let bytes = value.as_bytes();
    for i in 0..bytes.len() {
        match bytes[i] {
            b'(' => depth += 1,
            b')' => depth = depth.saturating_sub(1),
            b' ' if depth == 0 => {
                if i > start {
                    tokens.push(&value[start..i]);
                }
                start = i + 1;
            }
            _ => {}
        }
    }
    if value.len() > start {
        tokens.push(&value[start..]);
    }
    tokens
}

fn clean_value(value: &str) -> String {
    if value.starts_with('"') {
        let inner = &value[1..];
        let end = inner.find('"').unwrap_or(inner.len());
        return inner[..end].to_string();
    }
    // bare value: drop trailing comment
    let v = if let Some(pos) = value.find('#') { &value[..pos] } else { value };
    v.trim().to_string()
}

// ---------------------------------------------------------------------------
// The app-facing style model shared by every adapter. Values absent from
// shell.toml fall back to Omarchy's documented defaults (the commented lines
// in default/themed/shell.toml.tpl *are* the defaults).
// ---------------------------------------------------------------------------

/// Control-chrome state colors and alphas (the `[controls]` section).
#[derive(Clone, Copy, Debug)]
pub struct ControlState {
    pub color: Rgba,
    pub fill_alpha: f32,
    pub border_color: Option<Rgba>,
    pub border_width: f32,
    pub border_alpha: f32,
}

/// One card-like surface (menu, popup, tooltip, ...).
#[derive(Clone, Copy, Debug)]
pub struct Surface {
    pub background: Rgba,
    pub background_alpha: f32,
    pub text: Rgba,
    pub border: Option<Rgba>,
    pub border_alpha: f32,
    pub scrim: Option<Rgba>,
    pub scrim_alpha: f32,
}

/// The semantic style an adapter consumes: raw palette handled elsewhere;
/// this carries shape, alpha, and state behaviour.
#[derive(Clone, Debug)]
pub struct Style {
    /// True when a shell.toml was found and parsed.
    pub from_shell: bool,
    pub normal: ControlState,
    pub hover: ControlState,
    pub focus: ControlState,
    pub selected: ControlState,
    pub pressed_fill_alpha: f32,
    pub selection_fill_alpha: f32,
    pub menu: Option<Surface>,
    pub popup: Option<Surface>,
    pub tooltip: Option<Surface>,
    pub notification: Option<Surface>,
    /// `[hyprland]` active border first stop — the desktop's "you are here"
    /// accent used for focus rings and selection outlines.
    pub active_border: Option<Rgba>,
    /// `[font] base-size`: the rem root of the Omarchy type scale.
    pub font_base_size: f32,
    /// `[spacing] scale`: multiplier for margins/gaps/control dimensions.
    pub spacing_scale: f32,
    /// Raw spacing tokens (xxs..huge, control-gap, ...) when a shell.toml
    /// pinned them; None means "use defaults".
    pub spacing: BTreeMap<String, f64>,
}

impl Default for Style {
    fn default() -> Self {
        let fg = Rgba::WHITE;
        let mk = |fill: f32, border_alpha: f32| ControlState {
            color: fg,
            fill_alpha: fill,
            border_color: Some(fg),
            border_width: 1.0,
            border_alpha,
        };
        Style {
            from_shell: false,
            normal: mk(0.04, 0.4),
            hover: mk(0.08, 0.25),
            focus: mk(0.08, 0.25),
            selected: mk(0.18, 1.0),
            pressed_fill_alpha: 0.22,
            selection_fill_alpha: 0.35,
            menu: None,
            popup: None,
            tooltip: None,
            notification: None,
            active_border: None,
            font_base_size: 12.0,
            spacing_scale: 1.0,
            spacing: BTreeMap::new(),
        }
    }
}

impl Style {
    /// Build the app-facing style from parsed shell tokens and the palette
    /// (palette provides fallbacks for any token shell.toml omits).
    pub fn from_tokens(tokens: &StyleTokens, palette: &crate::Palette) -> Style {
        let mut s = Style::default();
        s.from_shell = !tokens.is_empty();
        // palette parameter is reserved for fallbacks of tokens newer than
        // this parser; every token defined today falls back to the template
        // defaults baked in below.
        let _ = palette;

        // [controls]
        let ctrl = |prefix: &str| -> Option<ControlState> {
            let color = tokens.color("controls", &format!("{prefix}-color"))?;
            Some(ControlState {
                color,
                fill_alpha: tokens
                    .number("controls", &format!("{prefix}-fill-alpha"))
                    .unwrap_or(0.0) as f32,
                border_color: tokens.color("controls", &format!("{prefix}-border")),
                border_width: tokens
                    .get("controls", &format!("{prefix}-border-width"))
                    .map(first_border_width)
                    .unwrap_or(0.0) as f32,
                border_alpha: tokens
                    .number("controls", &format!("{prefix}-border-alpha"))
                    .unwrap_or(1.0) as f32,
            })
        };
        if let Some(v) = ctrl("normal") { s.normal = v; }
        if let Some(v) = ctrl("hover-cursor") { s.hover = v; }
        if let Some(v) = ctrl("focus") { s.focus = v; }
        if let Some(v) = ctrl("selected") { s.selected = v; }
        s.pressed_fill_alpha = tokens
            .number("controls", "pressed-fill-alpha")
            .unwrap_or(0.22) as f32;
        s.selection_fill_alpha = tokens
            .number("controls", "selection-fill-alpha")
            .unwrap_or(0.35) as f32;

        // surfaces
        let surface = |section: &str| -> Option<Surface> {
            let background = tokens.color(section, "background")?;
            Some(Surface {
                background,
                background_alpha: tokens.number(section, "background-alpha").unwrap_or(1.0) as f32,
                text: tokens.color(section, "text").unwrap_or(background),
                border: tokens.color(section, "border"),
                border_alpha: tokens.number(section, "border-alpha").unwrap_or(1.0) as f32,
                scrim: tokens.color(section, "scrim"),
                scrim_alpha: tokens.number(section, "scrim-alpha").unwrap_or(0.5) as f32,
            })
        };
        s.menu = surface("menu");
        s.popup = surface("popups");
        s.tooltip = surface("tooltip");
        s.notification = surface("notifications");

        s.active_border = tokens.color("hyprland", "active-border");
        s.font_base_size = tokens.number("font", "base-size").unwrap_or(12.0) as f32;
        s.spacing_scale = tokens.number("spacing", "scale").unwrap_or(1.0) as f32;
        for (k, v) in tokens.iter_section("spacing") {
            if k == "scale" || k == "scale-with-font" {
                continue;
            }
            if let Ok(n) = v.parse::<f64>() {
                s.spacing.insert(k.to_string(), n);
            }
        }
        s
    }

    /// A spacing token with the shell's documented default table.
    pub fn spacing_token(&self, name: &str) -> f32 {
        if let Some(v) = self.spacing.get(name) {
            return (*v as f32) * self.spacing_scale;
        }
        let default: f32 = match name {
            "xxs" => 2.0,
            "xs" => 3.0,
            "sm" => 4.0,
            "md" => 6.0,
            "lg" => 8.0,
            "xl" => 10.0,
            "xxl" => 12.0,
            "xxxl" => 14.0,
            "huge" => 18.0,
            "control-gap" => 8.0,
            "control-padding-x" => 10.0,
            "control-padding-y" => 6.0,
            "input-padding-y" => 7.0,
            "control-height" => 28.0,
            "popup-row-height" => 28.0,
            "row-gap" => 8.0,
            "row-padding-x" => 12.0,
            "label-gap" => 4.0,
            "panel-gap" => 14.0,
            "panel-padding" => 18.0,
            "popup-padding" => 14.0,
            "dropdown-width" => 240.0,
            "searchable-dropdown-width" => 260.0,
            "number-field-width" => 120.0,
            "searchable-popup-min-height" => 220.0,
            _ => 0.0,
        };
        default * self.spacing_scale
    }
}

/// `"1"`, `"2 4"` -> leading scalar of a CSS-style border-width list.
fn first_border_width(value: &str) -> f64 {
    value
        .split_whitespace()
        .next()
        .and_then(|t| t.parse().ok())
        .unwrap_or(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r##"
[bar]
background       = "#060C1F"
background-alpha = 1.0

[hyprland]
active-border            = "#3FB4FF"
active-border-foreground = "#DCE6FF"

[controls]
normal-color        = "#DCE6FF"
normal-fill-alpha   = 0.04
normal-border       = "#DCE6FF"
normal-border-width = 1
normal-border-alpha = 0.4

[menu]
background       = "#060C1F"
background-alpha = 1.0
text             = "#DCE6FF"
border           = "hyprland.active-border-foreground"
scrim            = "#060C1F"
scrim-alpha      = 0.5

[font]
base-size = 12
"##;

    #[test]
    fn parses_flat_sections() {
        let t = StyleTokens::parse(SAMPLE);
        assert_eq!(t.get("bar", "background"), Some("#060C1F"));
        assert_eq!(t.number("bar", "background-alpha"), Some(1.0));
    }

    #[test]
    fn resolves_section_references() {
        let t = StyleTokens::parse(SAMPLE);
        assert_eq!(t.get("menu", "border"), Some("#DCE6FF"));
    }

    #[test]
    fn gradient_first_stop_flattening() {
        let t = StyleTokens::parse("[x]\nborder = \"rgba(0, 0, 0, 1) rgba(0, 0, 0, 0) 45deg\"\n");
        assert_eq!(t.color("x", "border"), Some(Rgba::BLACK));
        let t = StyleTokens::parse("[x]\nborder = \"#111111 #222222 90deg\"\n");
        assert_eq!(t.color("x", "border"), Rgba::parse("#111111"));
    }

    #[test]
    fn style_model_extracts_controls_and_surfaces() {
        let tokens = StyleTokens::parse(SAMPLE);
        let palette = crate::Palette::resolve("", false);
        let s = Style::from_tokens(&tokens, &palette);
        assert!(s.from_shell);
        assert_eq!(s.normal.fill_alpha, 0.04);
        assert_eq!(s.normal.border_alpha, 0.4);
        assert_eq!(s.menu.as_ref().unwrap().scrim_alpha, 0.5);
        assert_eq!(s.font_base_size, 12.0);
        assert_eq!(s.active_border, Rgba::parse("#3FB4FF"));
        assert_eq!(s.spacing_token("control-height"), 28.0);
    }

    #[test]
    fn defaults_apply_without_shell_toml() {
        let s = Style::default();
        assert_eq!(s.pressed_fill_alpha, 0.22);
        assert_eq!(s.hover.fill_alpha, 0.08);
    }
}
