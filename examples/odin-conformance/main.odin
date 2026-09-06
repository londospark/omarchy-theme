// omarchy_native — a dependency-free, single-file implementation of the
// Omarchy theme contract (docs/contract.md). This is the reference a
// handmade app (e.g. Schema-Spelunker) copies: no C ABI, no Rust, just file
// reads. It resolves colors.toml exactly like omarchy-theme-color.
//
// Two modes:
//   omarchy_native [colors.toml]                -> dump resolved TSV
//   omarchy_native --conformance <vectors-dir>  -> verify against vectors
package main

import "core:fmt"
import "core:os"
import "core:sort"
import "core:strings"
import "core:encoding/json"

Palette :: map[string]string

// ---------------------------------------------------------------- parsing

is_key_char :: proc(ch: u8) -> bool {
	if '0' <= ch && ch <= '9' { return true }
	if 'a' <= ch && ch <= 'z' { return true }
	if 'A' <= ch && ch <= 'Z' { return true }
	return ch == '_' || ch == '-'
}

is_value_char :: proc(ch: u8) -> bool {
	if is_key_char(ch) { return true }
	specials := "#(),._+/% -"
	for i in 0..<len(specials) {
		if specials[i] == ch { return true }
	}
	return false
}

extract_value :: proc(raw: string) -> string {
	for i in 0..<len(raw) {
		ch := raw[i]
		if ch == '"' || ch == '\'' {
			rest := raw[i + 1:]
			for j in 0..<len(rest) {
				if rest[j] == ch {
					return rest[:j]
				}
			}
			return rest
		}
	}
	return strings.trim_space(raw)
}

// One `key = value` per line; quote-tolerant; charset-filtered; last wins.
// Mirrors parse_colors_file in omarchy-theme-color.
parse_colors :: proc(text: string, out: ^Palette) {
	lines_src := text
	for line in strings.split_lines_iterator(&lines_src) {
		trimmed := strings.trim_space(line)
		eq := strings.index_byte(trimmed, '=')
		if eq < 0 do continue

		key_bytes: [dynamic]u8
		for i in 0..<eq {
			ch := trimmed[i]
			if ch == '"' || ch == '\'' || ch == ' ' { continue }
			append(&key_bytes, ch)
		}
		key := string(key_bytes[:])
		if len(key) == 0 || key[0] == '#' { continue }
		ok := true
		for i in 0..<len(key) {
			if !is_key_char(key[i]) { ok = false; break }
		}
		if !ok do continue

		value := extract_value(trimmed[eq + 1:])
		for i in 0..<len(value) {
			if !is_value_char(value[i]) { ok = false; break }
		}
		if !ok do continue

		out[key] = value
	}
}

// ------------------------------------------------------------- resolution

lookup :: proc(p: ^Palette, key: string) -> string {
	// Empty values are "absent", like the oracle's bash truthiness tests.
	v, ok := p[key]
	if !ok || len(v) == 0 do return ""
	return v
}

set :: proc(p: ^Palette, key: string, value: string) {
	if len(value) > 0 {
		p[key] = value
	}
}

alias :: proc(p: ^Palette, key, fallback: string) {
	if len(lookup(p, key)) > 0 do return
	set(p, key, lookup(p, fallback))
}

hex_digit :: proc(ch: u8) -> int {
	if '0' <= ch && ch <= '9' { return int(ch - '0') }
	if 'a' <= ch && ch <= 'f' { return int(ch - 'a') + 10 }
	if 'A' <= ch && ch <= 'F' { return int(ch - 'A') + 10 }
	return -1
}

hex_pair :: proc(s: string, i: int) -> int {
	h, l := hex_digit(s[1 + i]), hex_digit(s[2 + i])
	if h < 0 || l < 0 do return -1
	return h * 16 + l
}

// 24-bit RGB from #rrggbb (strict; matches the oracle's luminance regex)
hex_rgb :: proc(s: string) -> (r, g, b: int, ok: bool) {
	if len(s) != 7 || s[0] != '#' do return 0, 0, 0, false
	r, g, b = hex_pair(s, 0), hex_pair(s, 2), hex_pair(s, 4)
	if r < 0 || g < 0 || b < 0 do return 0, 0, 0, false
	return r, g, b, true
}

mix_channel :: proc(s, d: int, amount: f64) -> int {
	return int(f64(s) * (1.0 - amount) + f64(d) * amount + 0.5)
}

hex_nibble :: proc(d: int) -> u8 {
	if d < 10 { return u8('0') + u8(d) }
	return u8('a') + u8(d - 10)
}

byte_pair_hex :: proc(v: int) -> [2]u8 {
	return [2]u8{hex_nibble(v / 16), hex_nibble(v % 16)}
}

// Oracle mix: int(start*(1-a) + end*a + 0.5) per channel, amount a fraction.
mix_hex :: proc(a, b: string, amount: f64) -> string {
	ar, ag, ab, ok1 := hex_rgb(a)
	br, bg, bb, ok2 := hex_rgb(b)
	if !ok1 || !ok2 do return "#000000"
	r := byte_pair_hex(mix_channel(ar, br, amount))
	g := byte_pair_hex(mix_channel(ag, bg, amount))
	bl := byte_pair_hex(mix_channel(ab, bb, amount))
	buf: [7]u8
	buf[0] = '#'
	buf[1], buf[2] = r[0], r[1]
	buf[3], buf[4] = g[0], g[1]
	buf[5], buf[6] = bl[0], bl[1]
	res, _ := strings.clone(string(buf[:]), context.temp_allocator)
	return res
}

first_nonempty :: proc(vals: []string) -> string {
	for v in vals {
		if len(v) > 0 do return v
	}
	return ""
}

resolve_palette :: proc(p: ^Palette, light_marker: bool) {
	legacy := [][2]string{
		{"background", "bg"}, {"dark_background", "dark_bg"},
		{"darker_background", "darker_bg"}, {"lighter_background", "lighter_bg"},
		{"foreground", "fg"}, {"dark_foreground", "dark_fg"},
		{"light_foreground", "light_fg"}, {"bright_foreground", "bright_fg"},
	}
	for pair in legacy {
		alias(p, pair[0], pair[1])
	}

	alias(p, "background", "color0")
	alias(p, "foreground", "color7")
	v := lookup(p, "background")
	if len(v) > 0 do p["color0"] = v
	v = lookup(p, "foreground")
	if len(v) > 0 do p["color7"] = v

	ansi_semantic := [][2]string{
		{"red", "color1"}, {"green", "color2"}, {"yellow", "color3"},
		{"blue", "color4"}, {"magenta", "color5"}, {"cyan", "color6"},
		{"bright_red", "color9"}, {"bright_green", "color10"},
		{"bright_yellow", "color11"}, {"bright_blue", "color12"},
		{"bright_magenta", "color13"}, {"bright_cyan", "color14"},
	}
	for pair in ansi_semantic {
		alias(p, pair[0], pair[1])
	}
	alias(p, "magenta", "purple")
	alias(p, "bright_magenta", "bright_purple")

	if len(lookup(p, "light_foreground")) == 0 {
		set(p, "light_foreground", first_nonempty([]string{lookup(p, "color7"), lookup(p, "foreground")}))
	}
	if len(lookup(p, "bright_foreground")) == 0 {
		set(p, "bright_foreground", first_nonempty([]string{lookup(p, "color15"), lookup(p, "foreground")}))
	}
	set(p, "cursor", lookup(p, "bright_foreground"))
	if len(lookup(p, "lighter_background")) == 0 {
		set(p, "lighter_background", first_nonempty([]string{lookup(p, "color0"), lookup(p, "background")}))
	}
	if len(lookup(p, "dark_foreground")) == 0 {
		set(p, "dark_foreground", first_nonempty([]string{lookup(p, "color8"), lookup(p, "foreground")}))
	}
	if len(lookup(p, "muted")) == 0 {
		set(p, "muted", first_nonempty([]string{lookup(p, "color8"), lookup(p, "dark_foreground")}))
	}
	if len(lookup(p, "selection")) == 0 {
		set(p, "selection", first_nonempty([]string{
			lookup(p, "selection_background"), lookup(p, "color8"),
			lookup(p, "color0"), lookup(p, "background"),
		}))
	}
	alias(p, "selection_background", "selection")
	alias(p, "selection_foreground", "bright_foreground")
	alias(p, "orange", "yellow")
	if len(lookup(p, "brown")) == 0 {
		orange := lookup(p, "orange")
		if len(orange) == 0 do orange = "#000000"
		set(p, "brown", mix_hex(orange, "#000000", 0.5))
	}

	if len(lookup(p, "dark_background")) == 0 {
		bg := lookup(p, "background")
		if len(bg) == 0 do bg = "#000000"
		set(p, "dark_background", mix_hex(bg, "#000000", 0.25))
	}
	if len(lookup(p, "darker_background")) == 0 {
		bg := lookup(p, "background")
		if len(bg) == 0 do bg = "#000000"
		set(p, "darker_background", mix_hex(bg, "#000000", 0.5))
	}
	bright := [][2]string{
		{"bright_red", "red"}, {"bright_yellow", "yellow"}, {"bright_green", "green"},
		{"bright_cyan", "cyan"}, {"bright_blue", "blue"}, {"bright_magenta", "magenta"},
	}
	for pair in bright {
		if len(lookup(p, pair[0])) == 0 {
			base := lookup(p, pair[1])
			if len(base) == 0 do base = "#000000"
			set(p, pair[0], mix_hex(base, "#ffffff", 0.2))
		}
	}
	alias(p, "purple", "magenta")
	alias(p, "bright_purple", "bright_magenta")

	semantic_ansi := [][2]string{
		{"color0", "background"}, {"color1", "red"}, {"color2", "green"},
		{"color3", "yellow"}, {"color4", "blue"}, {"color5", "magenta"},
		{"color6", "cyan"}, {"color7", "foreground"}, {"color8", "muted"},
		{"color9", "bright_red"}, {"color10", "bright_green"},
		{"color11", "bright_yellow"}, {"color12", "bright_blue"},
		{"color13", "bright_magenta"}, {"color14", "bright_cyan"},
		{"color15", "bright_foreground"},
	}
	for pair in semantic_ansi {
		alias(p, pair[0], pair[1])
	}
	for pair in legacy {
		v := lookup(p, pair[0])
		if len(v) > 0 do p[pair[1]] = v
	}

	// mode cascade: mode key, theme_type key, light.mode marker, luminance
	mode := first_nonempty([]string{lookup(p, "mode"), lookup(p, "theme_type")})
	if len(mode) == 0 && light_marker {
		mode = "light"
	}
	if len(mode) == 0 {
		mode = "dark"
		r, g, b, ok := hex_rgb(lookup(p, "background"))
		if ok && r + g + b > 382 {
			mode = "light"
		}
	}
	p["theme_type"] = mode
}

// ---------------------------------------------------------------- output

sort_keys :: proc(p: ^Palette) -> []string {
	keys: [dynamic]string
	for k, val in p^ {
		if len(val) == 0 do continue
		append(&keys, k)
	}
	sort.quick_sort_proc(keys[:], proc(a, b: string) -> int {
		return strings.compare(a, b)
	})
	return keys[:]
}

dump :: proc(p: ^Palette) {
	for k in sort_keys(p) {
		fmt.printf("%s\t%s\n", k, p[k])
	}
}

state_dir :: proc() -> string {
	s := os.get_env("XDG_STATE_HOME", context.temp_allocator)
	if len(s) > 0 {
		return strings.concatenate({s, "/omarchy/current"}, context.temp_allocator)
	}
	h := os.get_env("HOME", context.temp_allocator)
	return strings.concatenate({h, "/.local/state/omarchy/current"}, context.temp_allocator)
}

resolve_text :: proc(text: string, marker: bool) -> Palette {
	p := make(Palette)
	parse_colors(text, &p)
	resolve_palette(&p, marker)
	return p
}

load_and_resolve :: proc(path: string) -> (Palette, bool) {
	text, err := os.read_entire_file(path, context.allocator)
	if err != os.ERROR_NONE {
		return {}, false
	}
	defer delete(text, context.allocator)
	dir := path
	if i := strings.last_index_byte(path, '/'); i >= 0 {
		dir = path[:i]
	}
	marker := false
	if probe, e2 := os.open(strings.concatenate({dir, "/light.mode"}, context.temp_allocator)); e2 == os.ERROR_NONE {
		os.close(probe)
		marker = true
	}
	return resolve_text(string(text), marker), true
}

Vector :: struct {
	theme: string,
	colors_file: string,
	light_mode_marker: bool,
	resolved: map[string]string,
}

dump_main :: proc() {
	// omarchy_native [colors.toml] — dump one resolved palette as TSV
	args := os.args
	path := strings.concatenate({state_dir(), "/theme/colors.toml"}, context.temp_allocator)
	if len(args) > 1 {
		path = args[1]
	}
	p, ok := load_and_resolve(path)
	if !ok {
		fmt.eprintfln("cannot read {}", path)
		os.exit(1)
	}
	dump(&p)
}

conformance_main :: proc() {
	// omarchy_native --conformance <vectors-dir>
	args := os.args
	assert(len(args) >= 3 && args[1] == "--conformance")
	dh, derr := os.open(args[2])
	if derr != os.ERROR_NONE {
		fmt.eprintfln("cannot list {}", args[2])
		os.exit(1)
	}
	entries, err := os.read_directory(dh, -1, context.temp_allocator)
	os.close(dh)
	if err != os.ERROR_NONE {
		fmt.eprintfln("cannot read {}", args[2])
		os.exit(1)
	}
	failures, checked := 0, 0
	for e in entries {
		if !strings.has_suffix(e.name, ".json") do continue
		path := strings.concatenate({args[2], "/", e.name}, context.temp_allocator)
		data, rerr := os.read_entire_file(path, context.allocator)
		if rerr != os.ERROR_NONE do continue
		v: Vector
		if jerr := json.unmarshal_string(string(data), &v, allocator = context.allocator); jerr != nil {
			fmt.eprintfln("bad vector {}: {}", path, jerr)
			os.exit(1)
		}
		delete(data, context.allocator)
		// sidecar colors are the committed, machine-independent truth
		src := strings.concatenate({args[2], "/", v.theme, ".colors.toml"}, context.temp_allocator)
		src_text, serr := os.read_entire_file(src, context.allocator)
		if serr != os.ERROR_NONE {
			src_text, serr = os.read_entire_file(v.colors_file, context.allocator)
			if serr != os.ERROR_NONE {
				fmt.printf("skip {} (source gone)\n", v.theme)
				continue
			}
		}
		p := resolve_text(string(src_text), v.light_mode_marker)
		defer delete(src_text, context.allocator)
		mismatches := 0
		for k, want in v.resolved {
			got := lookup(&p, k)
			if got != want {
				fmt.printf("FAIL {} [{}]: got `{}` want `{}`\n", v.theme, k, got, want)
				mismatches += 1
			}
		}
		for k in sort_keys(&p) {
			if _, have := v.resolved[k]; !have {
				fmt.printf("FAIL {} [{}]: extra key\n", v.theme, k)
				mismatches += 1
			}
		}
		if mismatches > 0 {
			failures += 1
		} else {
			fmt.printf("ok   {}\n", v.theme)
		}
		checked += 1
		delete(p)
	}
	fmt.printf("checked {} vectors, {} failed\n", checked, failures)
	code := 0
	if failures > 0 do code = 1
	os.exit(code)
}

main :: proc() {
	// Dispatch: --conformance <dir> verifies vectors, otherwise dump one theme.
	args := os.args
	if len(args) >= 2 && args[1] == "--conformance" {
		conformance_main()
	} else {
		dump_main()
	}
}
