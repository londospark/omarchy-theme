//! Resolve the live Omarchy desktop theme from Rust: the full palette (with
//! Omarchy's own alias/derivation cascade), shell style tokens, the UI font,
//! and the active background — plus two ways to notice the desktop theme
//! change.
//!
//! ```no_run
//! use omarchy_theme::Theme;
//! fn apply(theme: &Theme) { /* push to your toolkit here */ }
//!
//! let mut theme = Theme::current()?.with_font();
//! apply(&theme);
//!
//! // Poll tier: once per frame in an immediate-mode app loop.
//! if theme.changed() {
//!     theme.reload()?;
//!     apply(&theme);
//! }
//! # Ok::<(), omarchy_theme::Error>(())
//! ```
//!
//! See `docs/contract.md` in the repository for the language-neutral state
//! contract this crate implements; the conformance tests verify palette
//! resolution against the real `omarchy-theme-color` script for every
//! installed theme.

pub mod color;
pub mod error;
pub mod export;
pub mod font;
pub mod imgui_recipe;
pub mod palette;
pub mod style;
pub mod theme;
pub mod watch;

pub use color::Rgba;
pub use error::{Error, Result};
pub use export::Manifest;
pub use font::Font;
pub use palette::Palette;
pub use style::{Style, StyleTokens, Surface};
pub use theme::{current_state_dir, find_theme, theme_search_dirs, Signature, Theme, ThemeSource};
pub use watch::ThemeWatcher;

/// Light or dark, as decided by the palette's mode cascade.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Mode {
    #[default]
    Dark,
    Light,
}

impl Mode {
    pub fn as_str(self) -> &'static str {
        match self {
            Mode::Dark => "dark",
            Mode::Light => "light",
        }
    }

    pub fn is_light(self) -> bool {
        self == Mode::Light
    }
}

impl std::fmt::Display for Mode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Crate version, for `--version` plumbing and ABI checks.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The ABI version of the C interface exposed by `omarchy-theme-abi`.
pub const ABI_VERSION: u32 = 1;
