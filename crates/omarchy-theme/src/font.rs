//! Font resolution. Omarchy's `omarchy font set` configures fontconfig's
//! `monospace` family — fontconfig *is* the source of truth — so we ask it,
//! via `fc-match`, exactly like `omarchy-font-current` does, and additionally
//! resolve the font file path so GUI apps can load it into their own atlas
//! (ImGui, raylib, etc. do not go through fontconfig).

use std::path::PathBuf;
use std::process::Command;

/// The resolved UI font: family name and, when available, the file to load.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Font {
    pub family: String,
    pub file: Option<PathBuf>,
}

/// `fc-match monospace` — family first entry (fc-match prints an alias list),
/// mirroring `omarchy-font-current`.
pub fn resolve_ui_font() -> Option<Font> {
    let family = fc_match(&["monospace", "-f", "%{family}\n"])?;
    let family = family.split(',').next()?.trim().to_string();
    if family.is_empty() {
        return None;
    }
    let file = fc_match(&["monospace", "-f", "%{file}\n"])
        .map(PathBuf::from)
        .filter(|p| p.is_file());
    Some(Font { family, file })
}

fn fc_match(args: &[&str]) -> Option<String> {
    let out = Command::new("fc-match").args(args).output().ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8(out.stdout).ok()?;
    Some(s.trim_end().to_string())
}
