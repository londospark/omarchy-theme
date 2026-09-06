//! Locating and loading the live Omarchy theme, and detecting when it
//! changes. Everything reads the *staged* state under
//! `$XDG_STATE_HOME/omarchy/current/` (default `~/.local/state/...`), which
//! `omarchy-theme-set` swaps atomically — the contract in `docs/contract.md`.

use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{Error, Result};
use crate::palette::Palette;
use crate::style::{Style, StyleTokens};

/// The complete current theme: palette, shell style tokens, font, background.
#[derive(Clone, Debug)]
pub struct Theme {
    /// Slug from `current/theme.name` (e.g. "catppuccin"), if readable.
    pub name: Option<String>,
    pub palette: Palette,
    pub style: Style,
    pub tokens: StyleTokens,
    /// Resolved monospace UI font (fontconfig is the source of truth).
    pub font: Option<crate::font::Font>,
    /// Active wallpaper, target of `current/background`.
    pub background: Option<PathBuf>,
    /// The staged theme directory this was loaded from.
    pub dir: PathBuf,
    signature: Signature,
}

impl Theme {
    /// Load the theme the desktop is currently using.
    pub fn current() -> Result<Theme> {
        Self::from_state_dir(current_state_dir()?)
    }

    /// Load from an arbitrary Omarchy state root (the directory containing
    /// `theme/`, `theme.name`, `background`) — used by tests and by apps that
    /// want to resolve a non-active theme.
    pub fn from_state_dir(state_dir: impl AsRef<Path>) -> Result<Theme> {
        let state_dir = state_dir.as_ref();
        let theme_dir = state_dir.join("theme");
        if !theme_dir.is_dir() {
            return Err(Error::NoTheme);
        }
        let name = fs::read_to_string(state_dir.join("theme.name"))
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        let background = fs::read_link(state_dir.join("background"))
            .ok()
            .filter(|p| p.is_file());
        Self::load(theme_dir, name, background)
    }

    /// Load from a raw or staged theme directory (anywhere that holds a
    /// `colors.toml`), with no font lookup and no background resolution.
    pub fn from_theme_dir(dir: impl AsRef<Path>) -> Result<Theme> {
        let dir = dir.as_ref().to_path_buf();
        if !dir.join("colors.toml").is_file() {
            return Err(Error::NoTheme);
        }
        Self::load(dir, None, None)
    }

    fn load(dir: PathBuf, name: Option<String>, background: Option<PathBuf>) -> Result<Theme> {
        let colors_path = dir.join("colors.toml");
        let raw = fs::read_to_string(&colors_path)?;
        let has_light_marker = dir.join("light.mode").is_file();
        let palette = Palette::resolve(&raw, has_light_marker);
        let tokens = match fs::read_to_string(dir.join("shell.toml")) {
            Ok(s) => StyleTokens::parse(&s),
            Err(_) => StyleTokens::default(),
        };
        let style = Style::from_tokens(&tokens, &palette);
        let signature = Signature::capture(&dir)?;
        Ok(Theme { name, palette, style, tokens, font: None, background, dir, signature })
    }

    /// Attach the fontconfig-resolved UI font (spawns `fc-match`).
    pub fn with_font(mut self) -> Self {
        self.font = crate::font::resolve_ui_font();
        self
    }

    /// The observable identity of the theme this `Theme` was loaded with.
    pub fn signature(&self) -> &Signature {
        &self.signature
    }

    /// Cheap check (2-3 stats + one readlink): has anything about the theme
    /// changed since this `Theme` was loaded? Call once per frame from your
    /// loop; when true, call [`Theme::reload`].
    ///
    /// During a theme swap the staged directory is momentarily absent; that
    /// half-state deliberately reports *no change* so callers observe exactly
    /// one flip per desktop theme switch (contract: docs/contract.md §4).
    pub fn changed(&self) -> bool {
        if !self.dir.join("colors.toml").is_file() {
            return false; // swap in flight
        }
        match Signature::capture(&self.dir) {
            Ok(sig) => sig != self.signature,
            Err(_) => false,
        }
    }

    /// Re-read the theme in place; returns whether anything changed.
    pub fn reload(&mut self) -> Result<bool> {
        let fresh = Self::from_state_dir(
            self.dir
                .parent()
                .map(Path::to_path_buf)
                .unwrap_or_else(|| self.dir.clone()),
        )?;
        let changed = fresh.signature != self.signature;
        *self = fresh.with_font_optional(self.font.is_some());
        Ok(changed)
    }

    fn with_font_optional(self, want_font: bool) -> Self {
        if want_font {
            self.with_font()
        } else {
            self
        }
    }
}

/// The observable identity of the current theme: the *content* of the three
/// files that define appearance plus the background symlink target.
///
/// Content, not mtime: filesystem timestamp granularity (and tmpfs clock
/// coarsening) can make a swap burst write+rename within one timestamp tick.
/// Staging is atomic (render into a sibling directory, rename in), so reads
/// here always observe whole files.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Signature {
    name_file: Option<String>,
    colors: Option<String>,
    shell: Option<String>,
    background_target: Option<String>,
}

impl Signature {
    /// Snapshot the observable identity of a staged theme directory. Public
    /// so stateless pollers can compare signatures without holding a `Theme`.
    pub fn capture(theme_dir: &Path) -> Result<Signature> {
        let state_dir = theme_dir.parent().map(Path::to_path_buf).unwrap_or_default();
        Ok(Signature {
            name_file: read_opt(&state_dir.join("theme.name")),
            colors: read_opt(&theme_dir.join("colors.toml")),
            shell: read_opt(&theme_dir.join("shell.toml")),
            background_target: fs::read_link(state_dir.join("background"))
                .ok()
                .map(|p| p.to_string_lossy().into_owned()),
        })
    }
}

fn read_opt(path: &Path) -> Option<String> {
    fs::read_to_string(path).ok()
}

/// `$XDG_STATE_HOME` (default `~/.local/state`) joined with `omarchy/current`.
pub fn current_state_dir() -> Result<PathBuf> {
    let base = match std::env::var_os("XDG_STATE_HOME") {
        Some(v) if !v.is_empty() => PathBuf::from(v),
        _ => {
            let home = std::env::var_os("HOME")
                .ok_or_else(|| Error::Parse("HOME is unset".into()))?;
            PathBuf::from(home).join(".local/state")
        }
    };
    Ok(base.join("omarchy").join("current"))
}

/// The stock theme directory (`$OMARCHY_PATH/themes`, default
/// `/usr/share/omarchy/themes`) and the user theme directory.
pub fn theme_search_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Ok(v) = std::env::var("OMARCHY_PATH") {
        if !v.is_empty() {
            dirs.push(PathBuf::from(v).join("themes"));
        }
    }
    dirs.push(PathBuf::from("/usr/share/omarchy/themes"));
    if let Ok(home) = std::env::var("HOME") {
        dirs.push(PathBuf::from(home).join(".config/omarchy/themes"));
    }
    dirs.retain(|d| d.is_dir());
    dirs
}

/// Resolve an arbitrary theme by slug. `omarchy-theme-set` layers a user
/// theme of the same slug over the stock theme file-by-file, so each file is
/// taken from the user directory when present, else the stock directory.
pub fn find_theme(slug: &str) -> Option<ThemeSource> {
    if slug.is_empty() || slug.contains('/') || slug.contains('\\') || slug.starts_with('.') {
        return None;
    }
    let slug = slug.to_lowercase().replace(' ', "-");

    let candidate_roots: Vec<PathBuf> = ["OMARCHY_PATH", "HOME"]
        .iter()
        .filter_map(|v| std::env::var_os(v).filter(|s| !s.is_empty()))
        .map(PathBuf::from)
        .chain([PathBuf::from("/usr/share/omarchy")])
        .collect();

    let user = std::env::var_os("HOME")
        .filter(|s| !s.is_empty())
        .map(|h| PathBuf::from(h).join(".config/omarchy/themes").join(&slug))
        .filter(|d| d.is_dir());
    let stock = candidate_roots
        .iter()
        .map(|r| r.join("themes").join(&slug))
        .find(|d| d.is_dir());

    match (user, stock) {
        (None, None) => None,
        (u, s) => Some(ThemeSource { user: u, stock: s }),
    }
}

/// Where a requested theme's files live after user-over-stock layering.
#[derive(Clone, Debug)]
pub struct ThemeSource {
    /// `~/.config/omarchy/themes/<slug>` when it exists (overlay wins per
    /// file, exactly like `omarchy-theme-set` staging).
    pub user: Option<PathBuf>,
    /// The stock/`$OMARCHY_PATH` theme directory when it exists.
    pub stock: Option<PathBuf>,
}

impl ThemeSource {
    /// Resolve one theme file with user-over-stock precedence.
    pub fn file(&self, name: &str) -> Option<PathBuf> {
        self.user
            .as_ref()
            .map(|d| d.join(name))
            .filter(|p| p.is_file())
            .or_else(|| {
                self.stock
                    .as_ref()
                    .map(|d| d.join(name))
                    .filter(|p| p.is_file())
            })
    }

    /// Load this theme's palette without touching live state.
    pub fn load(&self) -> Result<Theme> {
        // Materialize a merge view: parse colors.toml from the layered file.
        let colors = self.file("colors.toml").ok_or(Error::NoTheme)?;
        let raw = fs::read_to_string(&colors)?;
        let has_light_marker = self.file("light.mode").is_some();
        let palette = Palette::resolve(&raw, has_light_marker);
        let tokens = match self.file("shell.toml") {
            Some(p) => StyleTokens::parse(&fs::read_to_string(p)?),
            None => StyleTokens::default(),
        };
        let style = Style::from_tokens(&tokens, &palette);
        let dir = colors.parent().unwrap_or(Path::new(".")).to_path_buf();
        Ok(Theme {
            name: None,
            palette,
            style,
            tokens,
            font: None,
            background: None,
            dir,
            signature: Signature::default(),
        })
    }
}
