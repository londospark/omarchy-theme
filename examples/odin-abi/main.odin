// Odin x omarchy-theme C ABI demo — doubles as the binding's conformance
// test. Loads the live theme through the shared library and prints the
// values a GUI app would use; check.sh diffs them against the Rust CLI.
//
// Build & run (workspace root):
//   ./examples/odin-abi/check.sh
package main

import "core:c"
import "core:fmt"
import "core:os"
import "core:strings"

import ot "omarchy:omarchy_theme"

hex_nib :: proc(d: u8) -> u8 {
	if d < 10 { return u8('0') + d }
	return u8('a') + d - 10
}

fmt_hex :: proc(v: u32) -> string {
	r := u8((v >> 24) & 0xFF)
	g := u8((v >> 16) & 0xFF)
	b := u8((v >> 8) & 0xFF)
	buf: [7]u8
	buf[0] = '#'
	buf[1] = hex_nib(r >> 4)
	buf[2] = hex_nib(r & 0xF)
	buf[3] = hex_nib(g >> 4)
	buf[4] = hex_nib(g & 0xF)
	buf[5] = hex_nib(b >> 4)
	buf[6] = hex_nib(b & 0xF)
	res, _ := strings.clone(string(buf[:]), context.temp_allocator)
	return res
}

mode_name :: proc(m: ot.Mode) -> string {
	if m == .Light { return "light" }
	return "dark"
}

main :: proc() {
	ot.check_abi()
	t, ok := ot.load()
	if !ok {
		fmt.eprintln("no live omarchy theme")
		os.exit(1)
	}
	fmt.printf("theme\t%s\n", ot.theme_name(&t))
	fmt.printf("mode\t%s\n", mode_name(t.mode))
	accent, _ := ot.slot_rgba(&t, .Accent)
	fmt.printf("accent\t%s\n", fmt_hex(ot.pack(accent)))
	bg, _ := ot.slot_rgba(&t, .Background)
	fmt.printf("background\t%s\n", fmt_hex(ot.pack(bg)))
	c0, _ := ot.ansi(&t, 0)
	fmt.printf("ansi0\t%s\n", fmt_hex(ot.pack(c0)))
	fmt.printf("font\t%s\n", ot.font_family(&t))
	fmt.printf("hover_fill_alpha\t%.4f\n", t.style[int(ot.Style_Slot.Hover_Fill_Alpha)])
	fmt.printf("changed\t%v\n", ot.changed()) // false right after load

	// get() reaches the raw resolved palette through the C boundary
	if v := ot.get("cursor"); len(v) > 0 {
		fmt.printf("get_cursor\t%s\n", v)
	}
}
