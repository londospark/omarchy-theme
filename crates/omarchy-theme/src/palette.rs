//! `colors.toml` parsing and the alias/derivation cascade.
//!
//! This module is a faithful, line-for-line port of the resolution order in
//! `/usr/share/omarchy/bin/omarchy-theme-color`, so that consumers resolve
//! byte-identical palettes on every machine. The conformance test suite
//! checks every installed theme against the real script; do not change this
//! file's order without regenerating `conformance/`.

use std::collections::{btree_map, BTreeMap};

use crate::color::{parse_mix_amount, Rgba};
use crate::Mode;

/// A fully resolved Omarchy palette: every raw key from `colors.toml` plus
/// every alias, ANSI slot, and derived shade, with the exact precedence the
/// desktop itself applies.
#[derive(Clone, Debug, Default)]
pub struct Palette {
    entries: BTreeMap<String, String>,
    mode: Mode,
}

impl Palette {
    /// Parse raw `colors.toml` text and run the full resolution cascade.
    ///
    /// `has_light_mode_marker` is true when a `light.mode` file sits beside
    /// `colors.toml`; it is only consulted when neither `mode` nor the legacy
    /// `theme_type` key is present.
    pub fn resolve(raw: &str, has_light_mode_marker: bool) -> Palette {
        let mut p = Palette {
            entries: parse_colors_text(raw),
            mode: Mode::Dark,
        };
        p.resolve_cascade(has_light_mode_marker);
        p
    }

    fn get_raw(&self, key: &str) -> Option<&str> {
        // Bash's `[[ ${map[k]} ]] ||` guard treats an empty value as absent;
        // mirror that exactly.
        match self.entries.get(key) {
            Some(v) if !v.is_empty() => Some(v.as_str()),
            _ => None,
        }
    }

    fn set(&mut self, key: &str, value: impl Into<String>) {
        let v = value.into();
        if !v.is_empty() {
            self.entries.insert(key.into(), v);
        }
    }

    /// `alias_theme_color`: assign `key` from `fallback` when `key` is unset.
    fn alias(&mut self, key: &str, fallback: &str) {
        if self.get_raw(key).is_none() {
            let v = self.get_raw(fallback).unwrap_or("").to_string();
            self.set(key, v);
        }
    }

    fn color_or(&self, key: &str) -> Rgba {
        self.color(key).unwrap_or(Rgba::BLACK)
    }

    fn resolve_cascade(&mut self, has_light_mode_marker: bool) {
        // --- legacy short names; canonical wins when both are present ---
        const LEGACY: &[(&str, &str)] = &[
            ("background", "bg"),
            ("dark_background", "dark_bg"),
            ("darker_background", "darker_bg"),
            ("lighter_background", "lighter_bg"),
            ("foreground", "fg"),
            ("dark_foreground", "dark_fg"),
            ("light_foreground", "light_fg"),
            ("bright_foreground", "bright_fg"),
        ];
        for (canonical, legacy) in LEGACY {
            self.alias(canonical, legacy);
        }

        // --- ANSI seed rules for pre-semantic themes ---
        if self.get_raw("background").is_none() {
            let v = self.get_raw("color0").unwrap_or("").to_string();
            self.set("background", v);
        }
        if self.get_raw("foreground").is_none() {
            let v = self.get_raw("color7").unwrap_or("").to_string();
            self.set("foreground", v);
        }
        if let Some(bg) = self.get_raw("background").map(str::to_string) {
            self.set("color0", bg);
        }
        if let Some(fg) = self.get_raw("foreground").map(str::to_string) {
            self.set("color7", fg);
        }

        // --- semantic <- ANSI names ---
        const ANSI_SEMANTIC: &[(&str, &str)] = &[
            ("red", "color1"),
            ("green", "color2"),
            ("yellow", "color3"),
            ("blue", "color4"),
            ("magenta", "color5"),
            ("cyan", "color6"),
            ("bright_red", "color9"),
            ("bright_green", "color10"),
            ("bright_yellow", "color11"),
            ("bright_blue", "color12"),
            ("bright_magenta", "color13"),
            ("bright_cyan", "color14"),
        ];
        for (key, fallback) in ANSI_SEMANTIC {
            self.alias(key, fallback);
        }
        self.alias("magenta", "purple");
        self.alias("bright_magenta", "bright_purple");

        // --- foreground/background ramp derivation (order-sensitive!) ---
        if self.get_raw("light_foreground").is_none() {
            let v = self
                .get_raw("color7")
                .or_else(|| self.get_raw("foreground"))
                .unwrap_or("")
                .to_string();
            self.set("light_foreground", v);
        }
        if self.get_raw("bright_foreground").is_none() {
            let v = self
                .get_raw("color15")
                .or_else(|| self.get_raw("foreground"))
                .unwrap_or("")
                .to_string();
            self.set("bright_foreground", v);
        }
        let bright = self.get_raw("bright_foreground").unwrap_or("").to_string();
        self.set("cursor", bright);
        if self.get_raw("lighter_background").is_none() {
            // Note: color0 was aliased from `background` above, so for themes
            // that define only `background` this resolves to that value —
            // matching the oracle's behaviour, quirks included.
            let v = self
                .get_raw("color0")
                .or_else(|| self.get_raw("background"))
                .unwrap_or("")
                .to_string();
            self.set("lighter_background", v);
        }
        if self.get_raw("dark_foreground").is_none() {
            let v = self
                .get_raw("color8")
                .or_else(|| self.get_raw("foreground"))
                .unwrap_or("")
                .to_string();
            self.set("dark_foreground", v);
        }
        if self.get_raw("muted").is_none() {
            let v = self
                .get_raw("color8")
                .or_else(|| self.get_raw("dark_foreground"))
                .unwrap_or("")
                .to_string();
            self.set("muted", v);
        }
        if self.get_raw("selection").is_none() {
            let v = self
                .get_raw("selection_background")
                .or_else(|| self.get_raw("color8"))
                .or_else(|| self.get_raw("color0"))
                .or_else(|| self.get_raw("background"))
                .unwrap_or("")
                .to_string();
            self.set("selection", v);
        }
        if self.get_raw("selection_background").is_none() {
            let v = self.get_raw("selection").unwrap_or("").to_string();
            self.set("selection_background", v);
        }
        if self.get_raw("selection_foreground").is_none() {
            let v = self.get_raw("bright_foreground").unwrap_or("").to_string();
            self.set("selection_foreground", v);
        }
        if self.get_raw("orange").is_none() {
            let v = self.get_raw("yellow").unwrap_or("").to_string();
            self.set("orange", v);
        }
        if self.get_raw("brown").is_none() {
            let orange = self.color_or("orange");
            self.set("brown", orange.mix(Rgba::BLACK, 0.5).to_hex());
        }

        // --- auto-derived shades ---
        if self.get_raw("dark_background").is_none() {
            let bg = self.color_or("background");
            self.set("dark_background", bg.mix(Rgba::BLACK, 0.25).to_hex());
        }
        if self.get_raw("darker_background").is_none() {
            let bg = self.color_or("background");
            self.set("darker_background", bg.mix(Rgba::BLACK, 0.5).to_hex());
        }
        for (key, base) in [
            ("bright_red", "red"),
            ("bright_yellow", "yellow"),
            ("bright_green", "green"),
            ("bright_cyan", "cyan"),
            ("bright_blue", "blue"),
            ("bright_magenta", "magenta"),
        ] {
            if self.get_raw(key).is_none() {
                let c = self.color_or(base);
                self.set(key, c.mix(Rgba::WHITE, 0.2).to_hex());
            }
        }
        self.alias("purple", "magenta");
        self.alias("bright_purple", "bright_magenta");

        // --- semantic -> ANSI ---
        const SEMANTIC_ANSI: &[(&str, &str)] = &[
            ("color0", "background"),
            ("color1", "red"),
            ("color2", "green"),
            ("color3", "yellow"),
            ("color4", "blue"),
            ("color5", "magenta"),
            ("color6", "cyan"),
            ("color7", "foreground"),
            ("color8", "muted"),
            ("color9", "bright_red"),
            ("color10", "bright_green"),
            ("color11", "bright_yellow"),
            ("color12", "bright_blue"),
            ("color13", "bright_magenta"),
            ("color14", "bright_cyan"),
            ("color15", "bright_foreground"),
        ];
        for (key, fallback) in SEMANTIC_ANSI {
            self.alias(key, fallback);
        }

        // --- canonical -> legacy short names (back-propagation) ---
        for (canonical, legacy) in LEGACY {
            if let Some(v) = self.get_raw(canonical).map(str::to_string) {
                self.set(legacy, v);
            }
        }

        self.mode = resolve_mode(self, has_light_mode_marker);
        let m = match self.mode {
            Mode::Dark => "dark",
            Mode::Light => "light",
        };
        self.set("theme_type", m);
    }

    /// The light/dark decision: `mode` key, legacy `theme_type`, a
    /// `light.mode` file beside the colors file, background-luminance
    /// auto-detect, then dark.
    pub fn mode(&self) -> Mode {
        self.mode
    }

    /// Resolved value for any palette key.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.get_raw(key)
    }

    /// Resolved key as a color; `None` when the value is not parseable
    /// (gradients and shell-specific specs are valid palette values but not
    /// colors).
    pub fn color(&self, key: &str) -> Option<Rgba> {
        self.get_raw(key).and_then(Rgba::parse)
    }

    /// ANSI slot 0..=15 as a color.
    pub fn ansi(&self, index: u8) -> Option<Rgba> {
        if index > 15 {
            return None;
        }
        self.color(&format!("color{index}"))
    }

    /// Mix two palette entries by key, e.g. `mix("background", "red", 0.15)`.
    pub fn mix(&self, from: &str, to: &str, amount: f64) -> Option<Rgba> {
        let a = self.color(from)?;
        let b = self.color(to)?;
        Some(a.mix(b, amount))
    }

    /// Mix a palette entry with a literal, mirroring the `{{ mix background
    /// red 15% }}` template helper. `amount` accepts `0.15`, `15`, or `"15%"`.
    pub fn mix_raw(&self, from: &str, to: &str, amount: &str) -> Option<Rgba> {
        let a = self.color(from)?;
        let b = Rgba::parse(to).or_else(|| self.color(to))?;
        Some(a.mix(b, parse_mix_amount(amount)))
    }

    /// Every resolved key, sorted (the `--all` keyset of the oracle).
    pub fn entries(&self) -> impl Iterator<Item = (&str, &str)> {
        self.entries.iter().map(|(k, v)| (k.as_str(), v.as_str()))
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

fn resolve_mode(palette: &Palette, has_light_mode_marker: bool) -> Mode {
    if let Some(m) = palette
        .get_raw("mode")
        .or_else(|| palette.get_raw("theme_type"))
    {
        return if m == "light" {
            Mode::Light
        } else {
            Mode::Dark
        };
    }
    if has_light_mode_marker {
        return Mode::Light;
    }
    if let Some(bg) = palette.get_raw("background") {
        // The oracle only auto-detects from a strict 6-digit hex background.
        let bytes = bg.as_bytes();
        if bytes.len() == 7 && bytes[0] == b'#' && bytes[1..].iter().all(|c| c.is_ascii_hexdigit())
        {
            if let Some(c) = Rgba::parse(bg) {
                return if c.is_light_by_omarchy() {
                    Mode::Light
                } else {
                    Mode::Dark
                };
            }
        }
    }
    Mode::Dark
}

/// The raw file parser from the oracle: one `key = value` per line,
/// quote-tolerant, charset-validated, last definition wins.
fn parse_colors_text(raw: &str) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    for line in raw.lines() {
        // IFS='=' read -r key value: split once on the first '='.
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };

        // strip all quote and space characters from the key
        let key: String = key
            .chars()
            .filter(|c| *c != '"' && *c != '\'' && *c != ' ')
            .collect();
        if key.is_empty() || key.starts_with('#') {
            continue;
        }

        let value = extract_value(value);

        let key_ok = !key.is_empty()
            && key
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-');
        if !key_ok {
            continue;
        }
        // Value charset: letters, digits, and #(),._+/% space, hyphen —
        // enough for hex, rgb()/rgba(), gradients, angles, and bare words.
        let value_ok = value.bytes().all(|c| {
            c.is_ascii_alphanumeric()
                || matches!(
                    c,
                    b'#' | b'(' | b')' | b',' | b'.' | b'_' | b'+' | b'/' | b'%' | b' ' | b'-'
                )
        });
        if !value_ok {
            continue;
        }

        map.insert(key, value);
    }
    map
}

/// Unquote/trim a raw value like the bash parameter expansions do: if quotes
/// are present take the text between the first quote and the next one;
/// otherwise trim surrounding whitespace.
fn extract_value(value: &str) -> String {
    if let Some(pos) = value.find(['\'', '"']) {
        let after = &value[pos + 1..];
        let end = after.find(['\'', '"']).unwrap_or(after.len());
        return after[..end].to_string();
    }
    value.trim().to_string()
}

impl IntoIterator for Palette {
    type Item = (String, String);
    type IntoIter = btree_map::IntoIter<String, String>;

    fn into_iter(self) -> Self::IntoIter {
        self.entries.into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quotes_and_comments_are_tolerated() {
        let p = Palette::resolve(
            "# comment = x\nmode = \"dark\"\naccent='#89b4fa' # inline\nselection = #45475a\n",
            false,
        );
        assert_eq!(p.get("mode"), Some("dark"));
        assert_eq!(p.get("accent"), Some("#89b4fa"));
        assert_eq!(p.get("selection"), Some("#45475a"));
        assert_eq!(p.get("comment"), None);
    }

    #[test]
    fn legacy_aliases_flow_both_ways() {
        let p = Palette::resolve("bg = #101010\nfg = #eeeeee\n", false);
        assert_eq!(p.get("background"), Some("#101010"));
        assert_eq!(p.get("foreground"), Some("#eeeeee"));
        // canonical -> legacy back-propagation
        assert_eq!(p.get("bg"), Some("#101010"));
        assert_eq!(p.get("color0"), Some("#101010"));
        assert_eq!(p.get("color7"), Some("#eeeeee"));
    }

    #[test]
    fn derived_shades_match_oracle_arithmetic() {
        let p = Palette::resolve(
            "background = #1e1e2e\nforeground = #cdd6f4\nred = #f38ba8\n",
            false,
        );
        assert_eq!(p.get("dark_background"), Some("#171723"));
        assert_eq!(p.get("darker_background"), Some("#0f0f17"));
        assert_eq!(p.get("bright_red"), Some("#f5a2b9")); // mix(red, white, 20%)
        assert_eq!(p.color("orange"), None); // no yellow and no color3 to alias
    }

    #[test]
    fn missing_orange_falls_back_to_yellow() {
        let p = Palette::resolve(
            "background = #000000\nforeground = #ffffff\nyellow = #f9e2af\n",
            false,
        );
        assert_eq!(p.get("orange"), Some("#f9e2af"));
        assert_eq!(p.get("brown"), Some("#7d7158"));
    }

    #[test]
    fn mode_detection_precedence() {
        let dark = Palette::resolve("mode = \"dark\"\nbackground = #ffffff\n", false);
        assert_eq!(dark.mode(), Mode::Dark);
        let legacy = Palette::resolve("theme_type = \"light\"\n", false);
        assert_eq!(legacy.mode(), Mode::Light);
        let marker = Palette::resolve("background = #101010\n", true);
        assert_eq!(marker.mode(), Mode::Light);
        let auto = Palette::resolve("background = #ffffff\n", false);
        assert_eq!(auto.mode(), Mode::Light);
        let auto_dark = Palette::resolve("background = #101010\n", false);
        assert_eq!(auto_dark.mode(), Mode::Dark);
    }

    #[test]
    fn empty_values_treated_as_absent() {
        let p = Palette::resolve("accent =\nforeground=#abcdef\n", false);
        assert_eq!(p.color("accent"), None);
    }
}
