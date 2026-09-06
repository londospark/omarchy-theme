//! C ABI for the omarchy-theme resolver.
//!
//! Design: one versioned, `repr(C)` POD struct filled by a copy (no
//! allocation crosses the boundary, nothing to free), a process-global cache
//! behind `omarchy_theme_load`/`omarchy_theme_changed` for the
//! one-theme-per-process reality of desktop apps, and slot-indexed color and
//! float arrays so the struct layout never churns as palettes grow.
//!
//! The contract mirrors `bindings/c/omarchy_theme.h`; the
//! `layout_matches_header` test keeps the two from drifting.

use std::ffi::{c_char, CStr};
use std::sync::Mutex;
use std::sync::OnceLock;

use omarchy_theme::{Rgba, Theme};

pub const OMARCHY_ABI_VERSION: u32 = 1;

// --- color slots (`colors[]`), packing 0xRRGGBBAA ---
pub const OMC_ACCENT: usize = 0;
pub const OMC_SELECTION: usize = 1;
pub const OMC_MUTED: usize = 2;
pub const OMC_BACKGROUND: usize = 3;
pub const OMC_DARK_BACKGROUND: usize = 4;
pub const OMC_DARKER_BACKGROUND: usize = 5;
pub const OMC_LIGHTER_BACKGROUND: usize = 6;
pub const OMC_FOREGROUND: usize = 7;
pub const OMC_DARK_FOREGROUND: usize = 8;
pub const OMC_LIGHT_FOREGROUND: usize = 9;
pub const OMC_BRIGHT_FOREGROUND: usize = 10;
pub const OMC_CURSOR: usize = 11;
pub const OMC_SELECTION_BACKGROUND: usize = 12;
pub const OMC_SELECTION_FOREGROUND: usize = 13;
pub const OMC_RED: usize = 14;
pub const OMC_YELLOW: usize = 15;
pub const OMC_ORANGE: usize = 16;
pub const OMC_GREEN: usize = 17;
pub const OMC_CYAN: usize = 18;
pub const OMC_BLUE: usize = 19;
pub const OMC_MAGENTA: usize = 20;
pub const OMC_BROWN: usize = 21;
pub const OMC_PURPLE: usize = 22;
pub const OMC_BRIGHT_RED: usize = 23;
pub const OMC_BRIGHT_YELLOW: usize = 24;
pub const OMC_BRIGHT_GREEN: usize = 25;
pub const OMC_BRIGHT_CYAN: usize = 26;
pub const OMC_BRIGHT_BLUE: usize = 27;
pub const OMC_BRIGHT_MAGENTA: usize = 28;
pub const OMC_BRIGHT_PURPLE: usize = 29;
pub const OMC_ACTIVE_BORDER: usize = 30;
pub const OMC_ANSI0: usize = 32; // ANSI0..ANSI15 are contiguous: OMC_ANSI0 + n
pub const OMARCHY_COLOR_SLOTS: usize = 64;

// --- style float slots (`style[]`); NaN when a token is unavailable ---
pub const OMS_NORMAL_FILL_ALPHA: usize = 0;
pub const OMS_NORMAL_BORDER_ALPHA: usize = 1;
pub const OMS_NORMAL_BORDER_WIDTH: usize = 2;
pub const OMS_HOVER_FILL_ALPHA: usize = 3;
pub const OMS_HOVER_BORDER_ALPHA: usize = 4;
pub const OMS_HOVER_BORDER_WIDTH: usize = 5;
pub const OMS_FOCUS_FILL_ALPHA: usize = 6;
pub const OMS_FOCUS_BORDER_ALPHA: usize = 7;
pub const OMS_FOCUS_BORDER_WIDTH: usize = 8;
pub const OMS_SELECTED_FILL_ALPHA: usize = 9;
pub const OMS_SELECTED_BORDER_ALPHA: usize = 10;
pub const OMS_SELECTED_BORDER_WIDTH: usize = 11;
pub const OMS_PRESSED_FILL_ALPHA: usize = 12;
pub const OMS_SELECTION_FILL_ALPHA: usize = 13;
pub const OMS_SPACING_SCALE: usize = 14;
pub const OMS_FONT_BASE_SIZE: usize = 15;
pub const OMS_MENU_BG_ALPHA: usize = 16;
pub const OMS_POPUP_BG_ALPHA: usize = 17;
pub const OMS_TOOLTIP_BG_ALPHA: usize = 18;
pub const OMS_SCRIM_ALPHA: usize = 19;
pub const OMARCHY_STYLE_SLOTS: usize = 32;

/// The POD theme snapshot. Fill `abi_size` with `sizeof(OmarchyTheme)` and
/// zero-init (or use `OMARCHY_THEME_INIT`) before calling load.
#[repr(C)]
pub struct OmarchyTheme {
    pub abi_size: u32,
    pub abi_version: u32,
    /// 0 = dark, 1 = light.
    pub mode: i32,
    pub has_font: u32,
    pub has_background: u32,
    pub reserved0: u32,
    pub theme_name: [c_char; 64],
    pub colors: [u32; OMARCHY_COLOR_SLOTS],
    pub style: [f32; OMARCHY_STYLE_SLOTS],
    pub font_family: [c_char; 64],
    pub font_path: [c_char; 512],
    pub background_path: [c_char; 512],
}

/// Palette color slots by name for the loader (index order must match the
/// constants above).
const PALETTE_SLOTS: &[&str] = &[
    "accent",
    "selection",
    "muted",
    "background",
    "dark_background",
    "darker_background",
    "lighter_background",
    "foreground",
    "dark_foreground",
    "light_foreground",
    "bright_foreground",
    "cursor",
    "selection_background",
    "selection_foreground",
    "red",
    "yellow",
    "orange",
    "green",
    "cyan",
    "blue",
    "magenta",
    "brown",
    "purple",
    "bright_red",
    "bright_yellow",
    "bright_green",
    "bright_cyan",
    "bright_blue",
    "bright_magenta",
    "bright_purple",
];

// --- process-global cache --------------------------------------------------

struct Cache {
    theme: Theme,
    entries: Vec<(String, String)>,
    signature: omarchy_theme::Signature,
}

fn cache() -> &'static Mutex<Option<Cache>> {
    static CACHE: OnceLock<Mutex<Option<Cache>>> = OnceLock::new();
    CACHE.get_or_init(Default::default)
}

fn copy_str<const N: usize>(dst: &mut [c_char; N], src: &str) {
    let bytes = src.as_bytes();
    let n = bytes.len().min(N - 1);
    for i in 0..n {
        dst[i] = bytes[i] as c_char;
    }
    dst[n] = 0;
}

fn rgba_slot(color: Option<Rgba>) -> u32 {
    color.map(Rgba::to_u32).unwrap_or(0) // alpha 0 = slot unavailable
}

fn fill(out: &mut OmarchyTheme, theme: &Theme) {
    out.abi_version = OMARCHY_ABI_VERSION;
    out.mode = match theme.palette.mode() {
        omarchy_theme::Mode::Dark => 0,
        omarchy_theme::Mode::Light => 1,
    };
    copy_str(&mut out.theme_name, theme.name.as_deref().unwrap_or(""));

    for (i, key) in PALETTE_SLOTS.iter().enumerate() {
        out.colors[i] = rgba_slot(theme.palette.color(key));
    }
    out.colors[OMC_ACTIVE_BORDER] = rgba_slot(theme.style.active_border);
    for i in 0..16 {
        out.colors[OMC_ANSI0 + i] = rgba_slot(theme.palette.ansi(i as u8));
    }

    let nan = f32::NAN;
    for slot in out.style.iter_mut() {
        *slot = nan;
    }
    let st = &theme.style;
    let mut put = |idx: usize, v: f32| out.style[idx] = v;
    put(OMS_NORMAL_FILL_ALPHA, st.normal.fill_alpha);
    put(OMS_NORMAL_BORDER_ALPHA, st.normal.border_alpha);
    put(OMS_NORMAL_BORDER_WIDTH, st.normal.border_width);
    put(OMS_HOVER_FILL_ALPHA, st.hover.fill_alpha);
    put(OMS_HOVER_BORDER_ALPHA, st.hover.border_alpha);
    put(OMS_HOVER_BORDER_WIDTH, st.hover.border_width);
    put(OMS_FOCUS_FILL_ALPHA, st.focus.fill_alpha);
    put(OMS_FOCUS_BORDER_ALPHA, st.focus.border_alpha);
    put(OMS_FOCUS_BORDER_WIDTH, st.focus.border_width);
    put(OMS_SELECTED_FILL_ALPHA, st.selected.fill_alpha);
    put(OMS_SELECTED_BORDER_ALPHA, st.selected.border_alpha);
    put(OMS_SELECTED_BORDER_WIDTH, st.selected.border_width);
    put(OMS_PRESSED_FILL_ALPHA, st.pressed_fill_alpha);
    put(OMS_SELECTION_FILL_ALPHA, st.selection_fill_alpha);
    put(OMS_SPACING_SCALE, st.spacing_scale);
    put(OMS_FONT_BASE_SIZE, st.font_base_size);
    put(OMS_MENU_BG_ALPHA, st.menu.map(|s| s.background_alpha).unwrap_or(nan));
    put(OMS_POPUP_BG_ALPHA, st.popup.map(|s| s.background_alpha).unwrap_or(nan));
    put(OMS_TOOLTIP_BG_ALPHA, st.tooltip.map(|s| s.background_alpha).unwrap_or(nan));
    put(OMS_SCRIM_ALPHA, st.menu.map(|s| s.scrim_alpha).unwrap_or(nan));

    out.has_font = 0;
    copy_str(&mut out.font_family, "");
    copy_str(&mut out.font_path, "");
    if let Some(font) = &theme.font {
        out.has_font = 1;
        copy_str(&mut out.font_family, &font.family);
        copy_str(&mut out.font_path, &font.file.as_ref().map(|p| p.display().to_string()).unwrap_or_default());
    }
    out.has_background = 0;
    copy_str(&mut out.background_path, "");
    if let Some(bg) = &theme.background {
        out.has_background = 1;
        copy_str(&mut out.background_path, &bg.display().to_string());
    }
}

fn load_into(out: &mut OmarchyTheme) -> i32 {
    let theme = match Theme::current() {
        Ok(t) => t.with_font(),
        Err(omarchy_theme::Error::NoTheme) => return -2,
        Err(_) => return -1,
    };
    let entries: Vec<(String, String)> =
        theme.palette.entries().map(|(k, v)| (k.to_string(), v.to_string())).collect();
    let signature = theme.signature().clone();
    fill(out, &theme);
    *cache().lock().unwrap() = Some(Cache { theme, entries, signature });
    0
}

/// Error codes for the `int32_t` returns.
pub const OMARCHY_OK: i32 = 0;
pub const OMARCHY_ERR: i32 = -1;
pub const OMARCHY_ERR_NO_THEME: i32 = -2;
pub const OMARCHY_ERR_ABI: i32 = -3;
pub const OMARCHY_ERR_ARG: i32 = -4;

// --- exported functions ----------------------------------------------------

/// ABI version implemented by this library (compare against
/// `OMARCHY_ABI_VERSION` from the header).
#[no_mangle]
pub extern "C" fn omarchy_theme_abi_version() -> u32 {
    OMARCHY_ABI_VERSION
}

/// Library version string ("1.2.3"), valid for the process lifetime.
#[no_mangle]
pub extern "C" fn omarchy_theme_version() -> *const c_char {
    static V: OnceLock<std::ffi::CString> = OnceLock::new();
    V.get_or_init(|| std::ffi::CString::new(omarchy_theme::VERSION).unwrap()).as_ptr()
}

/// Load the current theme into `out`. The caller must have set
/// `out->abi_size = sizeof(OmarchyTheme)`; the library fills only the fields
/// it knows (forward compatible: newer lib + older header works).
///
/// Returns `OMARCHY_OK` or a negative error code.
#[no_mangle]
pub unsafe extern "C" fn omarchy_theme_load(out: *mut OmarchyTheme) -> i32 {
    if out.is_null() {
        return OMARCHY_ERR_ARG;
    }
    let out = &mut *out;
    let ours = std::mem::size_of::<OmarchyTheme>() as u32;
    if out.abi_size == 0 || out.abi_size < 24 {
        return OMARCHY_ERR_ARG;
    }
    // A consumer with an older (smaller) struct still gets every field that
    // fits; we only promise layout for the fields it declared.
    let full = out.abi_size >= ours;
    if !full {
        // Older (smaller) consumer struct: fill a temp copy and hand over the
        // prefix — every field an old header knows lives before anything it
        // cannot have omitted. The consumer's `abi_size` field is overwritten
        // with the library's struct size (documented in the header).
        let mut tmp: OmarchyTheme = std::mem::zeroed();
        tmp.abi_size = ours;
        let rc = load_into(&mut tmp);
        if rc == OMARCHY_OK {
            let n = out.abi_size as usize;
            let src = &tmp as *const OmarchyTheme as *const u8;
            let dst = out as *mut OmarchyTheme as *mut u8;
            std::ptr::copy_nonoverlapping(src, dst, n);
            out.abi_size = n as u32; // keep the consumer's declared size intact
        }
        return rc;
    }
    load_into(out)
}

/// Poll tier: 1 if the desktop theme changed since the last successful
/// `omarchy_theme_load`, 0 otherwise, negative on error. Cheap (a few stats);
/// call once per frame. Deliberately silent during a swap's half-state, so
/// the flag rises exactly once per theme switch.
#[no_mangle]
pub extern "C" fn omarchy_theme_changed() -> i32 {
    let mut guard = cache().lock().unwrap();
    let Some(entry) = guard.as_mut() else {
        // Nothing loaded yet: report "changed" if a theme exists at all.
        return match Theme::current() {
            Ok(_) => 1,
            Err(_) => 0,
        };
    };
    if !entry.theme.dir.join("colors.toml").is_file() {
        return 0; // swap in flight
    }
    match Theme::current() {
        Ok(fresh) => u32::from(fresh.signature() != &entry.signature) as i32,
        Err(_) => 0,
    }
}

/// Resolved palette value (raw string: `#rrggbb`, gradients, ...) for a
/// key of the last loaded theme. Copies NUL-terminated output; returns the
/// length excluding NUL, or OMARCHY_ERR_* when missing/short buffer.
#[no_mangle]
pub unsafe extern "C" fn omarchy_theme_get(key: *const c_char, out: *mut c_char, out_len: u32) -> i32 {
    if key.is_null() || out.is_null() || out_len == 0 {
        return OMARCHY_ERR_ARG;
    }
    let Ok(key) = CStr::from_ptr(key).to_str() else { return OMARCHY_ERR_ARG };
    let guard = cache().lock().unwrap();
    let Some(entry) = guard.as_ref() else { return OMARCHY_ERR };
    let Some((_, value)) = entry.entries.iter().find(|(k, _)| k == key) else {
        return OMARCHY_ERR;
    };
    if value.len() + 1 > out_len as usize {
        return OMARCHY_ERR_ARG;
    }
    copy_str_raw(out, out_len as usize, value);
    value.len() as i32
}

/// Number of resolved palette entries in the last loaded theme (0 if none
/// loaded).
#[no_mangle]
pub extern "C" fn omarchy_theme_entry_count() -> u32 {
    cache().lock().unwrap().as_ref().map(|c| c.entries.len() as u32).unwrap_or(0)
}

/// Copy the `index`-th resolved palette entry (sorted by key). Returns the
/// value length excluding NUL, or a negative error.
#[no_mangle]
pub unsafe extern "C" fn omarchy_theme_entry(
    index: u32,
    key: *mut c_char,
    key_len: u32,
    value: *mut c_char,
    value_len: u32,
) -> i32 {
    if key.is_null() || value.is_null() || key_len == 0 || value_len == 0 {
        return OMARCHY_ERR_ARG;
    }
    let guard = cache().lock().unwrap();
    let Some(entry) = guard.as_ref() else { return OMARCHY_ERR };
    let Some((k, v)) = entry.entries.get(index as usize) else { return OMARCHY_ERR_ARG };
    if k.len() + 1 > key_len as usize || v.len() + 1 > value_len as usize {
        return OMARCHY_ERR_ARG;
    }
    copy_str_raw(key, key_len as usize, k);
    copy_str_raw(value, value_len as usize, v);
    v.len() as i32
}

fn copy_str_raw(dst: *mut c_char, cap: usize, src: &str) {
    let bytes = src.as_bytes();
    let n = bytes.len().min(cap - 1);
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr() as *const c_char, dst, n);
        *dst.add(n) = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_matches_header() {
        // bindings/c/omarchy_theme.h documents: size 1560, colors at 88,
        // style at 344. If these drift, bump OMARCHY_ABI_VERSION and update
        // the header + Odin bindings together.
        assert_eq!(std::mem::size_of::<OmarchyTheme>(), 1560);
        assert_eq!(std::mem::offset_of!(OmarchyTheme, colors), 88);
        assert_eq!(std::mem::offset_of!(OmarchyTheme, style), 344);
        assert_eq!(std::mem::offset_of!(OmarchyTheme, theme_name), 24);
        assert_eq!(std::mem::offset_of!(OmarchyTheme, font_family), 472);
        assert_eq!(std::mem::offset_of!(OmarchyTheme, font_path), 536);
        assert_eq!(std::mem::offset_of!(OmarchyTheme, background_path), 1048);
    }

    #[test]
    fn zeroed_abi_size_is_rejected() {
        let mut t: OmarchyTheme = unsafe { std::mem::zeroed() };
        assert_eq!(unsafe { omarchy_theme_load(&mut t) }, OMARCHY_ERR_ARG);
    }

    #[test]
    fn load_roundtrip_against_rust_core() {
        let Ok(theme) = Theme::current() else { return }; // needs live Omarchy
        let mut t: OmarchyTheme = unsafe { std::mem::zeroed() };
        t.abi_size = std::mem::size_of::<OmarchyTheme>() as u32;
        let rc = unsafe { omarchy_theme_load(&mut t) };
        assert_eq!(rc, OMARCHY_OK);
        assert_eq!(t.abi_version, OMARCHY_ABI_VERSION);
        assert_eq!(
            Rgba::from_u32(t.colors[OMC_ACCENT]),
            theme.palette.color("accent").unwrap(),
        );
        let name = unsafe { CStr::from_ptr(t.theme_name.as_ptr()) }.to_str().unwrap();
        assert_eq!(name, theme.name.as_deref().unwrap_or(""));
        // get() consults the global cache filled by the load above
        let mut buf = [0i8; 64];
        let n = unsafe {
            omarchy_theme_get(b"bg\0".as_ptr() as *const c_char, buf.as_mut_ptr(), 64)
        };
        assert!(n > 0);
    }

    #[test]
    fn older_smaller_header_still_loads() {
        #[repr(C)]
        struct Old {
            abi_size: u32,
            abi_version: u32,
            mode: i32,
            has_font: u32,
            has_background: u32,
            reserved0: u32,
            theme_name: [c_char; 64],
        }
        let Ok(_) = Theme::current() else { return };
        let mut old = unsafe { std::mem::zeroed::<Old>() };
        old.abi_size = std::mem::size_of::<Old>() as u32;
        let rc = unsafe { omarchy_theme_load(&mut old as *mut Old as *mut OmarchyTheme) };
        assert_eq!(rc, OMARCHY_OK);
        assert_eq!(old.abi_version, OMARCHY_ABI_VERSION);
    }
}
