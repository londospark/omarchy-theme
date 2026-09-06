// omarchy_theme — Odin bindings for the omarchy-theme C ABI.
//
// Build the library (workspace root):
//   cargo build --release -p omarchy-theme-abi
// Then link against target/release (libomarchy_theme_abi.so):
//   export LIBRARY_PATH=.../omarchy-theme/target/release
//   export LD_LIBRARY_PATH=.../omarchy-theme/target/release
//   odin build your_app -collection:omarchy-theme=<this dir>/../..
//
// The struct is the frozen version-1 POD from bindings/c/omarchy_theme.h;
// parity is asserted by tests in the Rust crate and by this package's own
// compile check against the library.
package omarchy_theme

import "core:c"
import "core:strings"

OMARCHY_ABI_VERSION :: 1

// Error codes
OMARCHY_OK :: 0
OMARCHY_ERR :: -1
OMARCHY_ERR_NO_THEME :: -2
OMARCHY_ERR_ABI :: -3
OMARCHY_ERR_ARG :: -4

Mode :: enum(i32) {
	Dark = 0,
	Light = 1,
}

// Color slots (indices into Theme.colors, packed 0xRRGGBBAA; alpha 0 means
// the slot could not be resolved).
Color_Slot :: enum(i32) {
	Accent = 0,
	Selection = 1,
	Muted = 2,
	Background = 3,
	Dark_Background = 4,
	Darker_Background = 5,
	Lighter_Background = 6,
	Foreground = 7,
	Dark_Foreground = 8,
	Light_Foreground = 9,
	Bright_Foreground = 10,
	Cursor = 11,
	Selection_Background = 12,
	Selection_Foreground = 13,
	Red = 14,
	Yellow = 15,
	Orange = 16,
	Green = 17,
	Cyan = 18,
	Blue = 19,
	Magenta = 20,
	Brown = 21,
	Purple = 22,
	Bright_Red = 23,
	Bright_Yellow = 24,
	Bright_Green = 25,
	Bright_Cyan = 26,
	Bright_Blue = 27,
	Bright_Magenta = 28,
	Bright_Purple = 29,
	Active_Border = 30,
}

// ANSI slot index for color0..15
Ansi_Index :: proc(i: int) -> int {
	assert(i >= 0 && i < 16)
	return 32 + i
}

// Style float slots (NaN when unavailable)
Style_Slot :: enum(i32) {
	Normal_Fill_Alpha = 0,
	Normal_Border_Alpha = 1,
	Normal_Border_Width = 2,
	Hover_Fill_Alpha = 3,
	Hover_Border_Alpha = 4,
	Hover_Border_Width = 5,
	Focus_Fill_Alpha = 6,
	Focus_Border_Alpha = 7,
	Focus_Border_Width = 8,
	Selected_Fill_Alpha = 9,
	Selected_Border_Alpha = 10,
	Selected_Border_Width = 11,
	Pressed_Fill_Alpha = 12,
	Selection_Fill_Alpha = 13,
	Spacing_Scale = 14,
	Font_Base_Size = 15,
	Menu_Bg_Alpha = 16,
	Popup_Bg_Alpha = 17,
	Tooltip_Bg_Alpha = 18,
	Scrim_Alpha = 19,
}

// POD layout frozen at 1560 bytes: 24 header, 64 name, 256 colors, 128
// style, 64+512+512 font/background. Asserted at startup by `check_abi`.
Theme :: struct {
	abi_size:       u32,
	abi_version:    u32,
	mode:           Mode,
	has_font:       u32,
	has_background: u32,
	reserved0:      u32,
	theme_name:     [64]c.char,
	colors:         [64]u32,
	style:          [32]f32,
	font_family:    [64]c.char,
	font_path:      [512]c.char,
	background_path: [512]c.char,
}

foreign import omarchy_theme_lib "system:omarchy_theme_abi"

foreign omarchy_theme_lib {
	omarchy_theme_abi_version :: proc() -> u32 ---
	omarchy_theme_version     :: proc() -> ^c.char ---
	omarchy_theme_load        :: proc(out: ^Theme) -> i32 ---
	omarchy_theme_changed     :: proc() -> i32 ---
	omarchy_theme_get         :: proc(key: ^c.char, out: ^c.char, out_len: u32) -> i32 ---
	omarchy_theme_entry_count :: proc() -> u32 ---
	omarchy_theme_entry       :: proc(index: u32, key: ^c.char, key_len: u32, value: ^c.char, value_len: u32) -> i32 ---
}

// --- friendly API -----------------------------------------------------------

// Load the current desktop theme. Zero-initialization and the abi_size
// handshake are handled here. ---
load :: proc() -> (t: Theme, ok: bool) {
	t = {}
	t.abi_size = size_of(Theme)
	rc := omarchy_theme_load(&t)
	return t, rc == OMARCHY_OK
}

// True when the desktop theme changed since the last successful load (cheap;
// call once per frame and re-load() when it is).
changed :: proc() -> bool {
	return omarchy_theme_changed() == 1
}

// Library version string.
library_version :: proc() -> string {
	return cstring_to_string(omarchy_theme_version())
}

// Verify size/offsets against what the linked library was built with.
// Panics with a clear message; useful in a debug startup path.
check_abi :: proc() {
	assert(size_of(Theme) == 1560, "omarchy_theme: Theme size drifted from ABI v1")
	assert(offset_of(Theme, colors) == 88, "omarchy_theme: colors offset drifted")
	assert(offset_of(Theme, style) == 344, "omarchy_theme: style offset drifted")
	assert(omarchy_theme_abi_version() == OMARCHY_ABI_VERSION, "omarchy_theme: library ABI mismatch")
}

// Resolved palette value by key (raw string; gradients included). Returns ""
// when absent, oversized (>255 chars), or not loaded. Keys longer than 63
// characters are rejected.
get :: proc(key: string) -> string {
	kbuf: [64]c.char
	if len(key) >= len(kbuf) do return ""
	for i := 0; i < len(key); i += 1 {
		kbuf[i] = c.char(key[i])
	}
	kbuf[len(key)] = 0
	buf: [256]c.char
	n := omarchy_theme_get(&kbuf[0], &buf[0], len(buf))
	if n < 0 do return ""
	// clone: the data lives in a stack buffer that dies with this call
	view := cstring_to_string(&buf[0])
	res, _ := strings.clone(view, context.temp_allocator)
	return res
}

// --- color helpers ----------------------------------------------------------

// Unpack a 0xRRGGBBAA slot.
Color :: struct { r, g, b, a: u8 }

unpack :: proc(packed: u32) -> Color {
	return {
		r = u8(packed >> 24),
		g = u8(packed >> 16),
		b = u8(packed >> 8),
		a = u8(packed),
	}
}

// 0xRRGGBBAA for ImGui (ColorEditor) or direct GPU use.
pack :: proc(c: Color) -> u32 {
	return u32(c.r) << 24 | u32(c.g) << 16 | u32(c.b) << 8 | u32(c.a)
}

// f32 rgba (0..1) for immediate-mode toolkit calls (ImVec4, raylib-style).
colorf :: proc(c: Color) -> [4]f32 {
	return {f32(c.r) / 255, f32(c.g) / 255, f32(c.b) / 255, f32(c.a) / 255}
}

slot_rgba :: proc(t: ^Theme, slot: Color_Slot) -> (c: Color, ok: bool) {
	v := t.colors[int(slot)]
	c = unpack(v)
	return c, c.a != 0
}

ansi :: proc(t: ^Theme, i: int) -> (c: Color, ok: bool) {
	assert(i >= 0 && i < 16)
	c = unpack(t.colors[32 + i])
	return c, c.a != 0
}

// Named strings from the POD.
theme_name :: proc(t: ^Theme) -> string {
	return cstring_to_string(&t.theme_name[0])
}
font_family :: proc(t: ^Theme) -> string {
	return cstring_to_string(&t.font_family[0])
}
font_path :: proc(t: ^Theme) -> string {
	return cstring_to_string(&t.font_path[0])
}
background_path :: proc(t: ^Theme) -> string {
	return cstring_to_string(&t.background_path[0])
}

// --- small internal helpers -------------------------------------------------


cstring_to_string :: proc(s: [^]c.char) -> string {
	if s == nil do return ""
	n := 0
	for s[n] != 0 {
		n += 1
	}
	if n == 0 do return ""
	mp: [^]u8 = s
	return string(mp[:n])
}
