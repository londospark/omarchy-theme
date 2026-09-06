// demo.odin — drives omarchy_theme_style.odin: prints the live Omarchy
// theme mapped into flat ImGui-style RGBA values, plus the generated
// .ssTheme text. Run it with the same args Schema-Spelunker's main would:
//   odin run . -out:build/spelunker-omarchy-demo
package main

import "core:fmt"
import "core:os"

print_col :: proc(tag: string, c: [4]f32) {
	fmt.printf("%-22s (%.3f, %.3f, %.3f) a=%.2f\n", tag, c[0], c[1], c[2], c[3])
}

main :: proc() {
	k, ok := load_omarchy_keys()
	if !ok {
		fmt.eprintln("no live Omarchy theme (not on Omarchy?)")
		os.exit(1)
	}
	t := theme_data_from_omarchy(&k)
	fmt.println("-- omarchy -> ImGui-style flat theme --")
	print_col("text_main", t.text_main)
	print_col("bg_window", t.bg_window)
	print_col("ctrl_frame", t.ctrl_frame)
	print_col("accent_colour", t.accent_colour)
	print_col("border_main", t.border_main)

	fmt.println("\n-- .ssTheme form --")
	ok2 := write_omarchy_ss_theme("/tmp/omarchy.ssTheme", "Omarchy (live)")
	if ok2 {
		fmt.println("wrote /tmp/omarchy.ssTheme")
	} else {
		fmt.println("failed to write")
	}

	fmt.println("\n-- change detection (poll tier) --")
	sig := ""
	fmt.println("changed? ", omarchy_theme_changed(&sig, context.temp_allocator))
	fmt.println("changed again? ", omarchy_theme_changed(&sig, context.temp_allocator))
}
