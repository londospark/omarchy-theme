//! zgui-verify: applies the Omarchy "mapping v1" recipe to a real Dear
//! ImGui context through zgui (zig-gamedev's toolkit) and asserts the
//! results against the live desktop theme's palette. This is the Zig
//! validation for the framework: bindings/zig/omarchy_theme.zig (C ABI) +
//! zgui + conformance of applied colors.
//!
//! Run:   zig build run           (prints verification)
//!        zig build test          (same, non-zero exit on mismatch)
const std = @import("std");
const zgui = @import("zgui");
const ot = @import("omarchy_theme");

fn f32eq(a: f32, b: f32) bool {
    return @abs(a - b) < 0.01;
}

/// "mapping v1" — the normative recipe (docs/contract.md §6), Zig edition.
/// Mirrors crates/omarchy-theme/src/imgui_recipe.rs and imgui.hpp.
fn applyOmarchy(style: *zgui.Style, t: *const ot.Theme) void {
    const col = struct {
        fn c(tt: *const ot.Theme, s: usize) ot.Color {
            return ot.colorFrom(tt, s);
        }
    }.c;

    const bg = col(t, ot.slot.background);
    const dbg = col(t, ot.slot.dark_background);
    const ddbg = col(t, ot.slot.darker_background);
    const lbg = col(t, ot.slot.lighter_background);
    const sel = col(t, ot.slot.selection);
    const fg = col(t, ot.slot.foreground);
    const dfg = col(t, ot.slot.dark_foreground);
    const bfg = col(t, ot.slot.bright_foreground);
    const muted = col(t, ot.slot.muted);
    const accent = col(t, ot.slot.accent);
    const active_border = ot.tryColorFrom(t, ot.slot.active_border) orelse accent;

    const hover_fill = ot.styleFloat(t, ot.style_slot.hover_fill_alpha, 0.08);
    const pressed_fill = ot.styleFloat(t, ot.style_slot.pressed_fill_alpha, 0.22);
    var base = ot.styleFloat(t, ot.style_slot.font_base_size, 12.0);
    if (base < 10.0) base = 10.0;
    var sc = ot.styleFloat(t, ot.style_slot.spacing_scale, 1.0);
    if (sc < 0.5) sc = 0.5;
    var window_alpha: f32 = ot.styleFloat(t, ot.style_slot.menu_bg_alpha, 0.95);
    if (window_alpha <= 0.0) window_alpha = 0.95;
    var popup_alpha: f32 = ot.styleFloat(t, ot.style_slot.popup_bg_alpha, 1.0);
    if (popup_alpha <= 0.0) popup_alpha = 1.0;

    const frame = bg.mix(lbg, 0.55);
    const frame_hover = frame.mix(fg, @max(hover_fill, 0.08));
    const frame_active = frame.mix(accent, @max(pressed_fill, 0.15));
    const button = frame.mix(accent, 0.10);
    const button_hover = button.mix(accent, 0.43);
    const button_active = button.mix(accent, 0.55);

    const colors = struct {
        fn put(s: *zgui.Style, comptime field: zgui.StyleCol, c: ot.Color) void {
            s.colors[@intCast(@intFromEnum(field))] = c.rgba();
        }
    }.put;

    colors(style, .text, fg);
    colors(style, .text_disabled, dfg);
    colors(style, .window_bg, bg.withAlpha(window_alpha));
    colors(style, .child_bg, .{ .r = 0, .g = 0, .b = 0, .a = 0 });
    colors(style, .popup_bg, bg.withAlpha(popup_alpha));
    colors(style, .border, muted);
    colors(style, .border_shadow, .{ .r = 0, .g = 0, .b = 0, .a = 0 });
    colors(style, .frame_bg, frame);
    colors(style, .frame_bg_hovered, frame_hover);
    colors(style, .frame_bg_active, frame_active);
    colors(style, .title_bg, dbg);
    colors(style, .title_bg_active, bg);
    colors(style, .title_bg_collapsed, dbg.withAlpha(0.6));
    colors(style, .menu_bar_bg, ddbg);
    colors(style, .scrollbar_bg, bg.withAlpha(0.0));
    colors(style, .scrollbar_grab, muted);
    colors(style, .scrollbar_grab_hovered, dfg);
    colors(style, .scrollbar_grab_active, accent);
    colors(style, .check_mark, accent);
    colors(style, .slider_grab, accent.mix(frame, 0.25));
    colors(style, .slider_grab_active, bfg);
    colors(style, .button, button);
    colors(style, .button_hovered, button_hover);
    colors(style, .button_active, button_active);
    colors(style, .header, bg.mix(accent, 0.12));
    colors(style, .header_hovered, bg.mix(accent, 0.25));
    colors(style, .header_active, bg.mix(accent, 0.40));
    colors(style, .separator, muted);
    colors(style, .separator_hovered, accent);
    colors(style, .separator_active, active_border);
    colors(style, .resize_grip, muted.withAlpha(0.35));
    colors(style, .resize_grip_hovered, accent);
    colors(style, .resize_grip_active, active_border);
    colors(style, .tab, ddbg.mix(fg, 0.06));
    colors(style, .tab_hovered, ddbg.mix(accent, 0.22));
    colors(style, .tab_selected, sel.mix(fg, 0.04));
    colors(style, .tab_selected_overline, accent);
    colors(style, .tab_dimmed, ddbg.withAlpha(0.6));
    colors(style, .tab_dimmed_selected, sel);
    colors(style, .tab_dimmed_selected_overline, accent);
    colors(style, .docking_preview, accent.withAlpha(0.4));
    colors(style, .docking_empty_bg, ddbg);
    colors(style, .plot_lines, accent);
    colors(style, .plot_lines_hovered, col(t, ot.slot.orange));
    colors(style, .plot_histogram, col(t, ot.slot.green));
    colors(style, .plot_histogram_hovered, col(t, ot.slot.bright_green));
    colors(style, .table_header_bg, bg.mix(lbg, 0.5));
    colors(style, .table_border_strong, muted);
    colors(style, .table_border_light, muted.withAlpha(0.5));
    colors(style, .table_row_bg, .{ .r = 0, .g = 0, .b = 0, .a = 0 });
    colors(style, .table_row_bg_alt, fg.withAlpha(0.03));
    colors(style, .text_selected_bg, sel);
    colors(style, .drag_drop_target, active_border);
    colors(style, .nav_cursor, active_border);
    colors(style, .nav_windowing_highlight, bfg.withAlpha(0.7));
    colors(style, .nav_windowing_dim_bg, ddbg.withAlpha(0.2));
    colors(style, .modal_window_dim_bg, ddbg.withAlpha(0.6));

    const round = struct {
        fn r(x: f32) f32 {
            return @floatFromInt(@as(i32, @intFromFloat(@min(1e9, @floor(x + 0.5)))));
        }
    }.r;
    const frame_round = round(base * 0.16);
    style.frame_rounding = frame_round;
    style.grab_rounding = frame_round;
    style.tab_rounding = frame_round;
    style.popup_rounding = round(base * 0.20);
    style.window_rounding = 0;
    style.child_rounding = 0;
    style.scrollbar_rounding = round(base * 0.45);
    style.window_border_size = 0;
    style.frame_border_size = 1;
    style.tab_border_size = 0;
    style.item_spacing = .{ round(6 * sc), round(4 * sc) };
    style.item_inner_spacing = .{ round(4 * sc), round(4 * sc) };
    style.frame_padding = .{ round(8 * sc), round(base * 0.25) };
    style.window_padding = .{ round(10 * sc), round(8 * sc) };
    style.scrollbar_size = round(base * 0.9);
    style.grab_min_size = round(base * 0.6);
}

/// Assert a applied color equals the expected Color (rgb within 1/255+eps).
fn expectColor(style: *const zgui.Style, field: zgui.StyleCol, want: ot.Color, what: []const u8) !void {
    const got = style.colors[@intCast(@intFromEnum(field))];
    const w = want.rgba();
    if (!(f32eq(got[0], w[0]) and f32eq(got[1], w[1]) and f32eq(got[2], w[2]))) {
        std.debug.print("MISMATCH {s}: got ({d:.3},{d:.3},{d:.3}) want ({d:.3},{d:.3},{d:.3})\n",
            .{ what, got[0], got[1], got[2], w[0], w[1], w[2] });
        return error.Mismatch;
    }
}

pub fn main() !void {
    const gpa = std.heap.c_allocator;

    // The bindings themselves must agree with the library ABI.
    if (ot.libraryAbi() != ot.abi_version) {
        std.debug.print("ABI version mismatch: lib {}\n", .{ot.libraryAbi()});
        std.process.exit(1);
    }

    var theme = ot.load() orelse {
        std.debug.print("zgui-verify: skip (no live Omarchy theme)\n", .{});
        return;
    };

    zgui.init(gpa);
    defer zgui.deinit();

    const style = zgui.getStyle();
    zgui.styleColorsDark(style); // deterministic baseline first
    applyOmarchy(style, &theme);

    try expectColor(style, .text, ot.colorFrom(&theme, ot.slot.foreground), "Text");
    try expectColor(style, .text_selected_bg, ot.colorFrom(&theme, ot.slot.selection), "TextSelectedBg");
    try expectColor(style, .border, ot.colorFrom(&theme, ot.slot.muted), "Border");
    try expectColor(style, .check_mark, ot.colorFrom(&theme, ot.slot.accent), "CheckMark");
    {
        const frame = ot.colorFrom(&theme, ot.slot.background)
            .mix(ot.colorFrom(&theme, ot.slot.lighter_background), 0.55);
        try expectColor(style, .button, frame.mix(ot.colorFrom(&theme, ot.slot.accent), 0.10), "Button");
    }
    if (style.window_rounding != 0 or style.window_border_size != 0) {
        std.debug.print("MISMATCH: compositor-owned shape scalars\n", .{});
        return error.Mismatch;
    }
    if (style.frame_border_size != 1) {
        std.debug.print("MISMATCH: frame_border_size\n", .{});
        return error.Mismatch;
    }

    // Also demonstrate the poll tier against the live theme, twice (must be
    // false right after a load).
    _ = &theme;
    if (ot.changed()) {
        if (ot.reload()) |fresh| {
            applyOmarchy(zgui.getStyle(), &fresh);
            theme = fresh;
        }
    }

    std.debug.print("zgui-verify: OK — theme '{s}' ({s}) applied to Dear ImGui via zgui\n", .{
        ot.themeName(&theme), if (theme.mode == .light) "light" else "dark",
    });
}
