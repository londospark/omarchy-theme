//! omarchy_native.zig — a single-file implementation of the Omarchy theme
//! contract (docs/contract.md), the Zig counterpart of the Odin reference.
//! Validated against the same conformance vectors.
//!
//!   zig run omarchy_native.zig -- [colors.toml]            dump resolved TSV
//!   zig run omarchy_native.zig -- --conformance <vectors>  verify vectors
const std = @import("std");

const Map = std.StringHashMap([]const u8);

// --------------------------------------------------------------- parsing

fn isKeyChar(c: u8) bool {
    return std.ascii.isAlphanumeric(c) or c == '_' or c == '-';
}

fn isValueChar(c: u8) bool {
    return isKeyChar(c) or c == '#' or c == '(' or c == ')' or c == ',' or
        c == '.' or c == '+' or c == '/' or c == '%' or c == ' ';
}

fn extractValue(raw: []const u8) []const u8 {
    for (raw, 0..) |q, i| {
        if (q == '"' or q == '\'') {
            const rest = raw[i + 1 ..];
            if (std.mem.indexOfScalar(u8, rest, q)) |j| return rest[0..j];
            return rest;
        }
    }
    return std.mem.trim(u8, raw, " \t\r\n");
}

fn parseColors(alloc: std.mem.Allocator, out: *Map, text: []const u8) !void {
    var lines = std.mem.splitScalar(u8, text, '\n');
    while (lines.next()) |line| {
        const trimmed = std.mem.trim(u8, line, " \t\r\n");
        const eq = std.mem.indexOfScalar(u8, trimmed, '=') orelse continue;

        var kbuf: [128]u8 = undefined;
        var klen: usize = 0;
        for (trimmed[0..eq]) |c| {
            if (c == '"' or c == '\'' or c == ' ') continue;
            if (klen == kbuf.len) break;
            kbuf[klen] = c;
            klen += 1;
        }
        const k = kbuf[0..klen];
        if (k.len == 0 or k[0] == '#') continue;
        var ok = true;
        for (k) |c| {
            if (!isKeyChar(c)) ok = false;
        }
        if (!ok) continue;

        const value = extractValue(trimmed[eq + 1 ..]);
        for (value) |c| {
            if (!isValueChar(c)) ok = false;
        }
        if (!ok) continue;

        try out.put(try alloc.dupe(u8, k), try alloc.dupe(u8, value));
    }
}

// -------------------------------------------------------------- mix rules

fn hexPair(s: []const u8, i: usize) ?u8 {
    const h = std.fmt.charToDigit(s[i], 16) catch return null;
    const l = std.fmt.charToDigit(s[i + 1], 16) catch return null;
    return h * 16 + l;
}

/// #rrggbb strict bytes; null otherwise (matches the oracle's regex).
fn hexRgb(s: []const u8) ?[3]u8 {
    if (s.len != 7 or s[0] != '#') return null;
    const r = hexPair(s, 1) orelse return null;
    const g = hexPair(s, 3) orelse return null;
    const b = hexPair(s, 5) orelse return null;
    return .{ r, g, b };
}

/// Oracle mix: int(start*(1-a) + end*a + 0.5) per channel, amount 0..1.
/// Writes lowercase #rrggbb into dst and returns the 7-byte slice.
fn mixHex(dst: *[8]u8, a: []const u8, b: []const u8, amount: f64) []const u8 {
    const from = hexRgb(a) orelse .{ 0, 0, 0 };
    const to = hexRgb(b) orelse .{ 0, 0, 0 };
    const ch = struct {
        fn f(s: u8, d: u8, amt: f64) u8 {
            return @intFromFloat(@min(255.0, @as(f64, @floatFromInt(s)) * (1.0 - amt) +
                @as(f64, @floatFromInt(d)) * amt + 0.5));
        }
    }.f;
    const hex = "0123456789abcdef";
    dst[0] = '#';
    for (0..3) |i| {
        const v = ch(from[i], to[i], amount);
        dst[1 + i * 2] = hex[v >> 4];
        dst[2 + i * 2] = hex[v & 15];
    }
    return dst[0..7];
}

fn get(p: *const Map, key: []const u8) ?[]const u8 {
    const v = p.get(key) orelse return null;
    if (v.len == 0) return null;
    return v;
}

fn firstOf(p: *const Map, keys: []const []const u8) ?[]const u8 {
    for (keys) |k| {
        if (get(p, k)) |v| return v;
    }
    return null;
}

fn set(p: *Map, alloc: std.mem.Allocator, key: []const u8, value: ?[]const u8) !void {
    const v = value orelse return;
    if (v.len == 0) return;
    try p.put(try alloc.dupe(u8, key), v);
}

fn alias(p: *Map, alloc: std.mem.Allocator, key: []const u8, fallback: []const u8) !void {
    if (get(p, key) != null) return;
    try set(p, alloc, key, get(p, fallback));
}

const AliasPair = struct { a: []const u8, b: []const u8 };

fn resolvePalette(alloc: std.mem.Allocator, p: *Map, light_marker: bool) !void {
    const legacy = [_]AliasPair{
        .{ .a = "background", .b = "bg" },
        .{ .a = "dark_background", .b = "dark_bg" },
        .{ .a = "darker_background", .b = "darker_bg" },
        .{ .a = "lighter_background", .b = "lighter_bg" },
        .{ .a = "foreground", .b = "fg" },
        .{ .a = "dark_foreground", .b = "dark_fg" },
        .{ .a = "light_foreground", .b = "light_fg" },
        .{ .a = "bright_foreground", .b = "bright_fg" },
    };
    for (legacy) |pr| try alias(p, alloc, pr.a, pr.b);

    try alias(p, alloc, "background", "color0");
    try alias(p, alloc, "foreground", "color7");
    if (get(p, "background")) |v| try p.put("color0", v);
    if (get(p, "foreground")) |v| try p.put("color7", v);

    const ansi_semantic = [_]AliasPair{
        .{ .a = "red", .b = "color1" },
        .{ .a = "green", .b = "color2" },
        .{ .a = "yellow", .b = "color3" },
        .{ .a = "blue", .b = "color4" },
        .{ .a = "magenta", .b = "color5" },
        .{ .a = "cyan", .b = "color6" },
        .{ .a = "bright_red", .b = "color9" },
        .{ .a = "bright_green", .b = "color10" },
        .{ .a = "bright_yellow", .b = "color11" },
        .{ .a = "bright_blue", .b = "color12" },
        .{ .a = "bright_magenta", .b = "color13" },
        .{ .a = "bright_cyan", .b = "color14" },
    };
    for (ansi_semantic) |pr| try alias(p, alloc, pr.a, pr.b);
    try alias(p, alloc, "magenta", "purple");
    try alias(p, alloc, "bright_magenta", "bright_purple");

    if (get(p, "light_foreground") == null)
        try set(p, alloc, "light_foreground", firstOf(p, &.{ "color7", "foreground" }));
    if (get(p, "bright_foreground") == null)
        try set(p, alloc, "bright_foreground", firstOf(p, &.{ "color15", "foreground" }));
    try set(p, alloc, "cursor", get(p, "bright_foreground"));
    if (get(p, "lighter_background") == null)
        try set(p, alloc, "lighter_background", firstOf(p, &.{ "color0", "background" }));
    if (get(p, "dark_foreground") == null)
        try set(p, alloc, "dark_foreground", firstOf(p, &.{ "color8", "foreground" }));
    if (get(p, "muted") == null)
        try set(p, alloc, "muted", firstOf(p, &.{ "color8", "dark_foreground" }));
    if (get(p, "selection") == null)
        try set(p, alloc, "selection", firstOf(p, &.{ "selection_background", "color8", "color0", "background" }));
    try alias(p, alloc, "selection_background", "selection");
    try alias(p, alloc, "selection_foreground", "bright_foreground");
    try alias(p, alloc, "orange", "yellow");

    var buf: [8]u8 = undefined;
    if (get(p, "brown") == null)
        try set(p, alloc, "brown", mixHex(&buf, get(p, "orange") orelse "#000000", "#000000", 0.5));
    if (get(p, "dark_background") == null)
        try set(p, alloc, "dark_background", mixHex(&buf, get(p, "background") orelse "#000000", "#000000", 0.25));
    if (get(p, "darker_background") == null)
        try set(p, alloc, "darker_background", mixHex(&buf, get(p, "background") orelse "#000000", "#000000", 0.5));

    const bright = [_]AliasPair{
        .{ .a = "bright_red", .b = "red" },
        .{ .a = "bright_yellow", .b = "yellow" },
        .{ .a = "bright_green", .b = "green" },
        .{ .a = "bright_cyan", .b = "cyan" },
        .{ .a = "bright_blue", .b = "blue" },
        .{ .a = "bright_magenta", .b = "magenta" },
    };
    for (bright) |pr| {
        if (get(p, pr.a) == null) {
            const base = get(p, pr.b) orelse "#000000";
            try set(p, alloc, pr.a, mixHex(&buf, base, "#ffffff", 0.2));
        }
    }
    try alias(p, alloc, "purple", "magenta");
    try alias(p, alloc, "bright_purple", "bright_magenta");

    const semantic_ansi = [_]AliasPair{
        .{ .a = "color0", .b = "background" },
        .{ .a = "color1", .b = "red" },
        .{ .a = "color2", .b = "green" },
        .{ .a = "color3", .b = "yellow" },
        .{ .a = "color4", .b = "blue" },
        .{ .a = "color5", .b = "magenta" },
        .{ .a = "color6", .b = "cyan" },
        .{ .a = "color7", .b = "foreground" },
        .{ .a = "color8", .b = "muted" },
        .{ .a = "color9", .b = "bright_red" },
        .{ .a = "color10", .b = "bright_green" },
        .{ .a = "color11", .b = "bright_yellow" },
        .{ .a = "color12", .b = "bright_blue" },
        .{ .a = "color13", .b = "bright_magenta" },
        .{ .a = "color14", .b = "bright_cyan" },
        .{ .a = "color15", .b = "bright_foreground" },
    };
    for (semantic_ansi) |pr| try alias(p, alloc, pr.a, pr.b);
    for (legacy) |pr| {
        if (get(p, pr.a)) |v| try p.put(pr.b, v);
    }

    var mode: []const u8 = get(p, "mode") orelse get(p, "theme_type") orelse "";
    if (mode.len == 0 and light_marker) mode = "light";
    if (mode.len == 0) {
        mode = "dark";
        if (get(p, "background")) |bgv| {
            if (hexRgb(bgv)) |rgb| {
                if (@as(u32, rgb[0]) + rgb[1] + rgb[2] > 382) mode = "light";
            }
        }
    }
    try p.put("theme_type", mode);
}

// ------------------------------------------------------------------ main

fn readFileText(io: std.Io, alloc: std.mem.Allocator, path: []const u8) ![]u8 {
    var file = std.Io.Dir.openFileAbsolute(io, path, .{}) catch return error.Missing;
    defer file.close(io);
    const st = try file.stat(io);
    const buf = try alloc.alloc(u8, st.size);
    const n = try file.readPositionalAll(io, buf, 0);
    return buf[0..n];
}

fn stateDirPath(alloc: std.mem.Allocator, env: *const std.process.Environ.Map) ![]const u8 {
    const base: []const u8 = blk: {
        if (env.get("XDG_STATE_HOME")) |s| {
            if (s.len > 0) break :blk s;
        }
        if (env.get("HOME")) |h| {
            break :blk try std.fmt.allocPrint(alloc, "{s}/.local/state", .{h});
        }
        break :blk "/tmp/.local/state";
    };
    return std.fmt.allocPrint(alloc, "{s}/omarchy/current/theme/colors.toml", .{base});
}

fn loadResolved(io: std.Io, alloc: std.mem.Allocator, p: *Map, colors_path: []const u8) !bool {
    const text = readFileText(io, alloc, colors_path) catch return false;

    const dir = std.fs.path.dirname(colors_path) orelse ".";
    const marker_path = try std.fmt.allocPrint(alloc, "{s}/light.mode", .{dir});
    var marker = false;
    if (std.Io.Dir.accessAbsolute(io, marker_path, .{})) {
        marker = true;
    } else |_| {}

    try parseColors(alloc, p, text);
    try resolvePalette(alloc, p, marker);
    return true;
}

fn writeOut(io: std.Io, bytes: []const u8) !void {
    var file = std.Io.File.stdout();
    try file.writeStreamingAll(io, bytes);
}

fn dump(io: std.Io, alloc: std.mem.Allocator, p: *const Map) !void {
    var out: std.ArrayList(u8) = .empty;
    var keys: std.ArrayList([]const u8) = .empty;
    var it = p.iterator();
    while (it.next()) |kv| {
        if (kv.value_ptr.len == 0) continue;
        try keys.append(alloc, kv.key_ptr.*);
    }
    std.mem.sort([]const u8, keys.items, {}, struct {
        fn lt(_: void, a: []const u8, b: []const u8) bool {
            return std.mem.order(u8, a, b).compare(.lt);
        }
    }.lt);
    for (keys.items) |k| {
        try out.print(alloc, "{s}\t{s}\n", .{ k, p.get(k).? });
    }
    try writeOut(io, out.items);
}

fn conformance(io: std.Io, alloc: std.mem.Allocator, dir_path: []const u8) !u8 {
    var failures: usize = 0;
    var checked: usize = 0;
    var out: std.ArrayList(u8) = .empty;

    var d = std.Io.Dir.openDirAbsolute(io, dir_path, .{ .iterate = true }) catch {
        std.debug.print("cannot open {s}\n", .{dir_path});
        return 1;
    };
    defer d.close(io);
    var it = d.iterate();
    while (try it.next(io)) |e| {
        if (e.kind != .file or !std.mem.endsWith(u8, e.name, ".json")) continue;
        const vpath = try std.fmt.allocPrint(alloc, "{s}/{s}", .{ dir_path, e.name });
        const vtext = readFileText(io, alloc, vpath) catch continue;

        const parsed = try std.json.parseFromSlice(std.json.Value, alloc, vtext, .{});
        const obj = switch (parsed.value) {
            .object => |o| o,
            else => continue,
        };

        var theme: []const u8 = "";
        var cf: []const u8 = "";
        var marker = false;
        var resolved = std.StringHashMap([]const u8).init(alloc);

        var oit = obj.iterator();
        while (oit.next()) |entry| {
            const k = entry.key_ptr.*;
            const v = entry.value_ptr.*;
            if (std.mem.eql(u8, k, "theme")) {
                theme = v.string;
            } else if (std.mem.eql(u8, k, "colors_file")) {
                cf = v.string;
            } else if (std.mem.eql(u8, k, "light_mode_marker")) {
                marker = v.bool;
            } else if (std.mem.eql(u8, k, "resolved")) {
                var rit = v.object.iterator();
                while (rit.next()) |re| {
                    try resolved.put(re.key_ptr.*, re.value_ptr.string);
                }
            }
        }

        var p = Map.init(alloc);
        const loaded = loadResolved(io, alloc, &p, cf) catch false;
        if (!loaded) {
            try out.print(alloc, "skip {s} (source gone)\n", .{theme});
            continue;
        }
        var mismatches: usize = 0;
        var rit2 = resolved.iterator();
        while (rit2.next()) |kv| {
            const got = get(&p, kv.key_ptr.*) orelse "";
            if (!std.mem.eql(u8, got, kv.value_ptr.*)) {
                try out.print(alloc, "FAIL {s} [{s}]: got {s} want {s}\n",
                    .{ theme, kv.key_ptr.*, got, kv.value_ptr.* });
                mismatches += 1;
            }
        }
        var pit = p.iterator();
        while (pit.next()) |kv| {
            if (!resolved.contains(kv.key_ptr.*)) {
                try out.print(alloc, "FAIL {s} [{s}]: extra\n", .{ theme, kv.key_ptr.* });
                mismatches += 1;
            }
        }
        if (mismatches > 0) {
            failures += 1;
        } else {
            try out.print(alloc, "ok   {s}\n", .{theme});
        }
        checked += 1;
    }
    try out.print(alloc, "checked {} vectors, {} failed\n", .{ checked, failures });
    try writeOut(io, out.items);
    return if (failures > 0) 1 else 0;
}

pub fn main(process: std.process.Init) !u8 {
    const io = process.io;
    const alloc = process.gpa;

    const argv = process.minimal.args.vector; // []const [*:0]const u8 on posix
    if (argv.len >= 3 and std.mem.eql(u8, std.mem.span(argv[1]), "--conformance")) {
        return conformance(io, alloc, std.mem.span(argv[2]));
    }

    var p = Map.init(alloc);
    const path = if (argv.len >= 2)
        std.mem.span(argv[1])
    else
        try stateDirPath(alloc, process.environ_map);
    if (!try loadResolved(io, alloc, &p, path)) {
        std.debug.print("cannot read {s}\n", .{path});
        return 1;
    }
    try dump(io, alloc, &p);
    return 0;
}
