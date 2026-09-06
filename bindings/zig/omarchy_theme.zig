//! omarchy_theme.zig — Zig bindings for the omarchy-theme C ABI (v1).
//!
//! Pure Zig `extern` declarations of the frozen POD layout from
//! bindings/c/omarchy_theme.h — no @cImport needed, just link the library:
//!
//!   zig build-exe app.zig -L<prefix>/lib -lomarchy_theme_abi
//!
//! (build.zig consumers: `exe.linkSystemLibrary("omarchy_theme_abi")` plus a
//! library search path, or vendor the static lib). The comptime assertions
//! below pin the exact struct layout, so header and bindings cannot drift
//! silently.
const std = @import("std");

pub const abi_version: u32 = 1;

pub const error_code = struct {
    pub const ok: i32 = 0;
    pub const err: i32 = -1;
    pub const no_theme: i32 = -2;
    pub const abi: i32 = -3;
    pub const arg: i32 = -4;
};

/// Color slots (indices into Theme.colors; values are 0xRRGGBBAA, alpha 0
/// means the slot could not be resolved).
pub const slot = struct {
    pub const accent: usize = 0;
    pub const selection: usize = 1;
    pub const muted: usize = 2;
    pub const background: usize = 3;
    pub const dark_background: usize = 4;
    pub const darker_background: usize = 5;
    pub const lighter_background: usize = 6;
    pub const foreground: usize = 7;
    pub const dark_foreground: usize = 8;
    pub const light_foreground: usize = 9;
    pub const bright_foreground: usize = 10;
    pub const cursor: usize = 11;
    pub const selection_background: usize = 12;
    pub const selection_foreground: usize = 13;
    pub const red: usize = 14;
    pub const yellow: usize = 15;
    pub const orange: usize = 16;
    pub const green: usize = 17;
    pub const cyan: usize = 18;
    pub const blue: usize = 19;
    pub const magenta: usize = 20;
    pub const brown: usize = 21;
    pub const purple: usize = 22;
    pub const bright_red: usize = 23;
    pub const bright_yellow: usize = 24;
    pub const bright_green: usize = 25;
    pub const bright_cyan: usize = 26;
    pub const bright_blue: usize = 27;
    pub const bright_magenta: usize = 28;
    pub const bright_purple: usize = 29;
    pub const active_border: usize = 30;
    pub const ansi_first: usize = 32;

    pub fn ansi(i: usize) usize {
        std.debug.assert(i < 16);
        return ansi_first + i;
    }
};

/// Style float slots (NaN when a token is unavailable).
pub const style_slot = struct {
    pub const normal_fill_alpha: usize = 0;
    pub const normal_border_alpha: usize = 1;
    pub const normal_border_width: usize = 2;
    pub const hover_fill_alpha: usize = 3;
    pub const hover_border_alpha: usize = 4;
    pub const hover_border_width: usize = 5;
    pub const focus_fill_alpha: usize = 6;
    pub const focus_border_alpha: usize = 7;
    pub const focus_border_width: usize = 8;
    pub const selected_fill_alpha: usize = 9;
    pub const selected_border_alpha: usize = 10;
    pub const selected_border_width: usize = 11;
    pub const pressed_fill_alpha: usize = 12;
    pub const selection_fill_alpha: usize = 13;
    pub const spacing_scale: usize = 14;
    pub const font_base_size: usize = 15;
    pub const menu_bg_alpha: usize = 16;
    pub const popup_bg_alpha: usize = 17;
    pub const tooltip_bg_alpha: usize = 18;
    pub const scrim_alpha: usize = 19;
};

pub const Mode = enum(i32) {
    dark = 0,
    light = 1,
};

/// POD snapshot of the current theme (frozen ABI v1, 1560 bytes).
pub const Theme = extern struct {
    abi_size: u32 = @sizeOf(Theme),
    abi_version: u32 = 0,
    mode: Mode = .dark,
    has_font: u32 = 0,
    has_background: u32 = 0,
    reserved0: u32 = 0,
    theme_name: [64]u8 = [_]u8{0} ** 64,
    colors: [64]u32 = [_]u32{0} ** 64,
    style: [32]f32 = [_]f32{std.math.nan(f32)} ** 32,
    font_family: [64]u8 = [_]u8{0} ** 64,
    font_path: [512]u8 = [_]u8{0} ** 512,
    background_path: [512]u8 = [_]u8{0} ** 512,
};

comptime {
    // Frozen v1 layout (must match omarchy_theme.h + the Rust ABI crate).
    std.debug.assert(@sizeOf(Theme) == 1560);
    std.debug.assert(@offsetOf(Theme, "theme_name") == 24);
    std.debug.assert(@offsetOf(Theme, "colors") == 88);
    std.debug.assert(@offsetOf(Theme, "style") == 344);
    std.debug.assert(@offsetOf(Theme, "font_family") == 472);
    std.debug.assert(@offsetOf(Theme, "font_path") == 536);
    std.debug.assert(@offsetOf(Theme, "background_path") == 1048);
}

extern fn omarchy_theme_abi_version() callconv(.c) u32;
extern fn omarchy_theme_version() callconv(.c) [*:0]const u8;
extern fn omarchy_theme_load(out: *Theme) callconv(.c) i32;
extern fn omarchy_theme_changed() callconv(.c) i32;
extern fn omarchy_theme_get(key: [*:0]const u8, out: [*]u8, out_len: u32) callconv(.c) i32;
extern fn omarchy_theme_entry_count() callconv(.c) u32;
extern fn omarchy_theme_entry(index: u32, key: [*]u8, key_len: u32, value: [*]u8, value_len: u32) callconv(.c) i32;

/// Load the current desktop theme.
pub fn load() ?Theme {
    var t = Theme{};
    t.abi_size = @sizeOf(Theme);
    if (omarchy_theme_load(&t) != error_code.ok) return null;
    return t;
}

/// Re-load the current theme (same as `load`; named for loop clarity).
pub fn reload() ?Theme {
    return load();
}

/// True when the desktop theme changed since the last successful load.
pub fn changed() bool {
    return omarchy_theme_changed() == 1;
}

pub fn libraryVersion() []const u8 {
    return std.mem.span(omarchy_theme_version());
}

pub fn libraryAbi() u32 {
    return omarchy_theme_abi_version();
}

// ---------------------------------------------------------------- color API

pub const Color = struct {
    r: u8,
    g: u8,
    b: u8,
    a: u8,

    pub fn unpack(v: u32) Color {
        return .{
            .r = @truncate(v >> 24),
            .g = @truncate(v >> 16),
            .b = @truncate(v >> 8),
            .a = @truncate(v),
        };
    }

    pub fn pack(c: Color) u32 {
        return (@as(u32, c.r) << 24) | (@as(u32, c.g) << 16) | (@as(u32, c.b) << 8) | c.a;
    }

    pub fn rgba(c: Color) [4]f32 {
        return .{
            @as(f32, @floatFromInt(c.r)) / 255.0,
            @as(f32, @floatFromInt(c.g)) / 255.0,
            @as(f32, @floatFromInt(c.b)) / 255.0,
            @as(f32, @floatFromInt(c.a)) / 255.0,
        };
    }

    pub fn rgb(c: Color) [3]f32 {
        return .{ c.rgba()[0], c.rgba()[1], c.rgba()[2] };
    }

    /// Oracle-identical per-channel mix (int linear + 0.5 floor), amount 0..1.
    pub fn mix(from: Color, to: Color, amount: f32) Color {
        const a = @min(1.0, @max(0.0, amount));
        const ch = struct {
            fn f(s: u8, d: u8, amt: f32) u8 {
                return @intFromFloat(@min(255.0, @as(f32, @floatFromInt(s)) * (1.0 - amt) +
                    @as(f32, @floatFromInt(d)) * amt + 0.5));
            }
        }.f;
        return .{
            .r = ch(from.r, to.r, a),
            .g = ch(from.g, to.g, a),
            .b = ch(from.b, to.b, a),
            .a = ch(from.a, to.a, a),
        };
    }

    pub fn withAlpha(c: Color, alpha: f32) Color {
        return .{ .r = c.r, .g = c.g, .b = c.b, .a = @intFromFloat(@min(255.0, @max(0.0, alpha) * 255.0 + 0.5)) };
    }
};

pub fn colorFrom(t: *const Theme, s: usize) Color {
    return Color.unpack(t.colors[s]);
}

/// Returns null when the slot was unresolvable (alpha == 0).
pub fn tryColorFrom(t: *const Theme, s: usize) ?Color {
    const c = colorFrom(t, s);
    if (c.a == 0 and c.pack() == 0) return null;
    return c;
}

pub fn styleFloat(t: *const Theme, s: usize, fallback: f32) f32 {
    const v = t.style[s];
    if (std.math.isNan(v)) return fallback;
    return v;
}

pub fn themeName(t: *const Theme) []const u8 {
    return strSpan(&t.theme_name);
}
pub fn fontFamily(t: *const Theme) []const u8 {
    return strSpan(&t.font_family);
}
pub fn fontPath(t: *const Theme) []const u8 {
    return strSpan(&t.font_path);
}
pub fn backgroundPath(t: *const Theme) []const u8 {
    return strSpan(&t.background_path);
}

fn strSpan(buf: []const u8) []const u8 {
    if (std.mem.indexOfScalar(u8, buf, 0)) |z| return buf[0..z];
    return buf;
}

// ----------------------------------------------------------------- palette

/// Raw resolved palette value by key (allocated copy; caller owns `buf`
/// semantics via the fixed buffer). Returns null if absent.
pub fn getValue(buf: []u8, key: [*:0]const u8) ?[]const u8 {
    const n = omarchy_theme_get(key, buf.ptr, @intCast(buf.len));
    if (n < 0) return null;
    return buf[0..@intCast(n)];
}

/// Enumerate every resolved palette entry (sorted by key). The callback
/// receives slices borrowed from buffers that stay valid only during the
/// call. Entries too long for the fixed buffers are skipped.
pub fn forEachEntry(comptime callback: fn (key: []const u8, value: []const u8) void) void {
    const count = omarchy_theme_entry_count();
    var i: u32 = 0;
    while (i < count) : (i += 1) {
        var key_buf: [128]u8 = undefined;
        var val_buf: [256]u8 = undefined;
        const n = omarchy_theme_entry(i, &key_buf, key_buf.len, &val_buf, val_buf.len);
        if (n < 0) continue;
        callback(std.mem.sliceTo(&key_buf, 0), val_buf[0..@intCast(n)]);
    }
}
