// omarchy_theme_style.odin — Omarchy live-theme integration for
// Schema-Spelunker (or any ImGui-style Odin app with a flat theme struct).
//
// Copy this file into the repo (next to style.odin) and wire two spots:
//
//   1. startup:   omarchy_style := load_omarchy_theme()
//                 data := theme_data_from_omarchy(omarchy_style)
//                 apply_theme(data)            // existing style.odin proc
//
//   2. frame top: if omarchy_theme_changed() {
//                     s := load_omarchy_theme()
//                     apply_theme(theme_data_from_omarchy(s))
//                 }
//
// That's the whole integration: no new binaries, no C library — it reads
// the Omarchy state contract (docs/contract.md) directly, in the same
// spirit as parse_ssTheme. Also exposes write_omarchy_ss_theme() to emit an
// .ssTheme file if you'd rather have "Omarchy" appear in the theme menu
// (hook/omarchy-theme-gen can refresh it on every desktop switch).

package main

import "core:fmt"
import "core:mem"
import "core:os"
import "core:strconv"
import "core:strings"

// ---------------------------------------------------------------------------
// Resolution state (contract §2/§3 of docs/contract.md)
// ---------------------------------------------------------------------------

Omarchy_Keys :: struct {
	mode, accent, selection, muted: string,
	background, dark_background, darker_background, lighter_background: string,
	foreground, dark_foreground, light_foreground, bright_foreground: string,
	red, yellow, orange, green, cyan, blue, magenta, brown: string,
	bright_red, bright_yellow, bright_green, bright_cyan, bright_blue, bright_magenta: string,
	ansi:                 [16]string,
	active_border:        string,  // shell.toml [hyprland] first stop
	hover_fill_alpha:     f32,
	pressed_fill_alpha:   f32,
}

omarchy_state_dir :: proc() -> string {
	if s := os.get_env("XDG_STATE_HOME", context.temp_allocator); len(s) > 0 {
		return strings.concatenate({s, "/omarchy/current"}, context.temp_allocator)
	}
	h := os.get_env("HOME", context.temp_allocator)
	return strings.concatenate({h, "/.local/state/omarchy/current"}, context.temp_allocator)
}

// read_kv_file parses the flat `key = value` dialect used by colors.toml and
// the generated shell.toml (sections tolerated: they only nest keys, which
// palette keys don't use; for shell.toml we pick specific keys explicitly).
read_kv_file :: proc(path: string, alloc: mem.Allocator) -> (map[string]string, bool) {
	data, err := os.read_entire_file(path, alloc)
	if err != os.ERROR_NONE do return nil, false
	m := make(map[string]string, 32)
	src := string(data)
	for line in strings.split_lines_iterator(&src) {
		trimmed := strings.trim_space(line)
		if len(trimmed) == 0 || trimmed[0] == '#' || trimmed[0] == '[' do continue
		eq := strings.index_byte(trimmed, '=')
		if eq < 0 do continue
		key := strings.trim_space(trimmed[:eq])
		val := strings.trim_space(trimmed[eq + 1:])
		if len(val) >= 2 && val[0] == '"' {
			if e := strings.index_byte(val[1:], '"'); e >= 0 {
				val = val[1 : e + 1]
			}
		} else if h := strings.index_byte(val, '#'); h > 0 {
			val = strings.trim_space(val[:h])
		}
		if len(key) > 0 do m[key] = val
	}
	return m, true
}

// pick applies the contract cascade for the handful of keys the theme needs
// on top of a fully-resolved staged file (staged themes carry every key).
load_omarchy_keys :: proc() -> (k: Omarchy_Keys, ok: bool) {
	alloc := context.temp_allocator
	dir := omarchy_state_dir()
	colors, cok := read_kv_file(strings.concatenate({dir, "/theme/colors.toml"}, alloc), alloc)
	if !cok do return {}, false
	defer delete(colors)

	g := proc(m: map[string]string, key: string) -> string {
		v, have := m[key]
		if !have || len(v) == 0 { return "#000000" }
		return v
	}

	k.mode = g(colors, "mode")
	k.accent = g(colors, "accent")
	k.selection = g(colors, "selection")
	k.muted = g(colors, "muted")
	k.background = g(colors, "background")
	k.dark_background = g(colors, "dark_background")
	k.darker_background = g(colors, "darker_background")
	k.lighter_background = g(colors, "lighter_background")
	k.foreground = g(colors, "foreground")
	k.dark_foreground = g(colors, "dark_foreground")
	k.light_foreground = g(colors, "light_foreground")
	k.bright_foreground = g(colors, "bright_foreground")
	k.red = g(colors, "red")
	k.yellow = g(colors, "yellow")
	k.orange = g(colors, "orange")
	k.green = g(colors, "green")
	k.cyan = g(colors, "cyan")
	k.blue = g(colors, "blue")
	k.magenta = g(colors, "magenta")
	k.brown = g(colors, "brown")
	k.bright_red = g(colors, "bright_red")
	k.bright_yellow = g(colors, "bright_yellow")
	k.bright_green = g(colors, "bright_green")
	k.bright_cyan = g(colors, "bright_cyan")
	k.bright_blue = g(colors, "bright_blue")
	k.bright_magenta = g(colors, "bright_magenta")
	for i in 0..<16 {
		k.ansi[i] = g(colors, fmt.tprintf("color{}", i))
	}

	// shell.toml: only three tokens are needed for a faithful feel
	if shell, sok := read_kv_file(strings.concatenate({dir, "/theme/shell.toml"}, alloc), alloc); sok {
		defer delete(shell)
		// the [hyprland] active border (first stop if a gradient)
		if v, have := shell["active-border"]; have {
			k.active_border = first_token(v)
		} else {
			k.active_border = k.accent
		}
		if v, have := shell["hover-cursor-fill-alpha"]; have {
			k.hover_fill_alpha = parse_f32_or(v, 0.08)
		} else {
			k.hover_fill_alpha = 0.08
		}
		if v, have := shell["pressed-fill-alpha"]; have {
			k.pressed_fill_alpha = parse_f32_or(v, 0.22)
		} else {
			k.pressed_fill_alpha = 0.22
		}
	} else {
		k.active_border = k.accent
		k.hover_fill_alpha = 0.08
		k.pressed_fill_alpha = 0.22
	}
	return k, true
}

first_token :: proc(s: string) -> string {
	if i := strings.index_byte(s, ' '); i > 0 { return s[:i] }
	return s
}

parse_f32_or :: proc(s: string, fallback: f32) -> f32 {
	v, ok := strconv.parse_f32(s)
	if !ok { return fallback }
	return v
}

// ---------------------------------------------------------------------------
// Change detection (contract §4, poll tier)
// ---------------------------------------------------------------------------

omarchy_signature :: proc(alloc: mem.Allocator) -> string {
	dir := omarchy_state_dir()
	name, _ := os.read_entire_file(strings.concatenate({dir, "/theme.name"}, alloc), alloc)
	colors, _ := os.read_entire_file(strings.concatenate({dir, "/theme/colors.toml"}, alloc), alloc)
	shell, _ := os.read_entire_file(strings.concatenate({dir, "/theme/shell.toml"}, alloc), alloc)
	bg, _ := os.read_link(strings.concatenate({dir, "/background"}, alloc), alloc)
	return fmt.tprintf("{}|{}|{}|{}", string(name), string(colors), string(shell), string(bg))
}

// Call once per frame with a frame-static signature; true means reload.
omarchy_theme_changed :: proc(last: ^string, alloc: mem.Allocator) -> bool {
	sig := omarchy_signature(alloc)
	if len(last^) == 0 {
		cloned, _ := strings.clone(sig, alloc)
		last^ = cloned
		return false
	}
	if sig == last^ { return false }
	cloned, _ := strings.clone(sig, alloc)
	last^ = cloned
	return true
}

// ---------------------------------------------------------------------------
// Mapping to a flat ImGui-style theme struct
// ---------------------------------------------------------------------------

// Adjust the field names to the app's own ThemeData and you're done.
Omarchy_ImGui_Theme :: struct {
	text_main: [4]f32,
	text_muted: [4]f32,
	bg_window, bg_child, bg_popup: [4]f32,
	ctrl_frame, ctrl_frame_hover, ctrl_frame_active: [4]f32,
	title_bg, title_bg_focus, title_bg_faded: [4]f32,
	card_bg, card_outline: [4]f32,
	border_main, border_subtle: [4]f32,
	grid_bg, grid_line: [4]f32,
	link_color: [4]f32,
	accent_colour: [4]f32,
	font_family: string,
}

// byte-pair hex component:
hex_comp :: proc(s: string, i: int) -> f32 {
	digit :: proc(ch: u8) -> int {
		if '0' <= ch && ch <= '9' { return int(ch - '0') }
		if 'a' <= ch && ch <= 'f' { return int(ch - 'a') + 10 }
		if 'A' <= ch && ch <= 'F' { return int(ch - 'A') + 10 }
		return 0
	}
	return f32(digit(s[i]) * 16 + digit(s[i + 1])) / 255.0
}

col :: proc(s: string, alpha: f32 = 1.0) -> [4]f32 {
	if len(s) != 7 || s[0] != '#' { return {0, 0, 0, alpha} }
	return {hex_comp(s, 1), hex_comp(s, 3), hex_comp(s, 5), alpha}
}

// mix two #rrggbb with the oracle's half-up per-channel rounding
mix :: proc(a, b: string, amount: f32) -> [4]f32 {
	ca, cb := col(a), col(b)
	t :: proc(x, y, amt: f32) -> f32 {
		v := int(x * 255.0 * (1 - amt) + y * 255.0 * amt + 0.5)
		return f32(v) / 255.0
	}
	return {t(ca[0], cb[0], amount), t(ca[1], cb[1], amount),
			t(ca[2], cb[2], amount), 1.0}
}

// theme_data_from_omarchy maps the resolved palette onto a flat ImGui-style
// theme using the "mapping v1" recipe (docs/contract.md §6).
theme_data_from_omarchy :: proc(k: ^Omarchy_Keys) -> Omarchy_ImGui_Theme {
	frame := mix(k.background, k.lighter_background, 0.55)
	return Omarchy_ImGui_Theme{
		text_main = col(k.foreground),
		text_muted = col(k.dark_foreground),
		bg_window = col(k.background, 0.95),
		bg_child = col(k.background, 0.95),
		bg_popup = col(k.background, 1.0),
		ctrl_frame = frame,
		ctrl_frame_hover = mix(k.background, k.lighter_background, 0.75),
		ctrl_frame_active = mix(k.lighter_background, k.accent, k.pressed_fill_alpha),
		title_bg = col(k.darker_background),
		title_bg_focus = col(k.background),
		title_bg_faded = col(k.dark_background, 0.75),
		card_bg = col(k.lighter_background),
		card_outline = col(k.muted),
		border_main = col(k.muted),
		border_subtle = col(k.selection),
		grid_bg = col(k.dark_background),
		grid_line = col(k.darker_background),
		link_color = col(k.blue),
		accent_colour = col(k.accent),
	}
}

// ---------------------------------------------------------------------------
// Optional: emit the .ssTheme file form (for theme-menu integration)
// ---------------------------------------------------------------------------

write_omarchy_ss_theme :: proc(path, name: string) -> bool {
	k, ok := load_omarchy_keys()
	if !ok do return false
	content := fmt.tprintf(
		"# generated from the live Omarchy theme - regenerate on theme-set\n" +
		"name = \"%s\"\n\n" +
		"[text]\nmain = %s\nmuted = %s\n\n" +
		"[background]\nwindow = %s\nchild = %s\npopup = %s\n\n" +
		"[controls]\nframe = %s\nframe_hover = %s\nframe_active = %s\n\n" +
		"[border]\nmain = %s\nsubtle = %s\n\n" +
		"[accent]\ncolour = %s\n",
		name,
		k.foreground, k.dark_foreground,
		k.background, k.background, k.background,
		k.lighter_background, k.selection, k.muted,
		k.muted, k.selection,
		k.accent,
	)
	return os.write_entire_file(path, content) == os.ERROR_NONE
}
