# Recipe: Zig (C ABI bindings + zgui, or the native contract loader)

Zig has two first-class paths, both validated in this repo
(`tests/check-zig.sh`):

## A. C ABI bindings (`bindings/zig/omarchy_theme.zig`) — recommended

Pure Zig `extern` declarations of the frozen v1 POD struct (comptime size/
offset asserts; no `@cImport`, just link). Works with *any* Zig toolkit —
validated here with [zgui](https://github.com/zig-gamedev/zgui) (zig-gamedev's
Dear ImGui package) in `examples/zgui-verify/`:

```zig
const ot = @import("omarchy_theme");

var theme = ot.load().?;          // live Omarchy theme through libomarchy_theme_abi
for (0..100_000) |_| {            // your frame loop
    if (ot.changed()) theme = ot.reload().?;   // poll tier, one line
    const accent = ot.colorFrom(&theme, ot.slot.accent);
    style.colors[@intFromEnum(zgui.StyleCol.button)] =
        frame.mix(accent, 0.10).rgba();        // "mapping v1", see contract §6
}
```

build.zig wiring (zig 0.16, module-centric linking):

```zig
mod.addLibraryPath(b.path("path/to/omarchy-theme/target/release"));
mod.linkSystemLibrary("omarchy_theme_abi", .{});
mod.addRPath(.{ .cwd_relative = b.pathFromRoot("path/to/omarchy-theme/target/release") });
mod.link_libc = true;
exe.use_llvm = true; exe.use_lld = true; // ziglang/zig#31272 workaround on glibc 2.43+
```

Full working example: `examples/zgui-verify/` — it initializes a real zgui
context, applies the theme with the recipe port, and *asserts* the resulting
ImGui style colors against the live palette.

## B. Native contract loader (`examples/zig-conformance/omarchy_native.zig`)

~350 lines, zero linking: reads `$XDG_STATE_HOME/omarchy/current/theme/`
and resolves the palette itself, exactly per `docs/contract.md`. Passes the
same 24-vector conformance suite. Copy it into a handmade app and build the
mapping from `Theme`-style keys yourself — no shared library at runtime.

## Which to pick

- Using an ImGui-style toolkit (zgui, zigimgui) or anything that redraws
  from style data → **A** (you get shell.toml control alphas, font, and
  background paths for free).
- Want zero runtime dependency and your theme is flat colors → **B**.
- Non-ImGui Zig toolkits (egui-zig, slint-zig, ...): consume **A**'s POD and
  write your own 20-line mapping — the struct is toolkit-neutral on purpose.

## Poll vs push in Zig

The C ABI is poll-only (`omarchy_theme_changed()`), matching how every Zig
GUI toolkit runs (you own the frame loop). For push semantics (event-driven
apps), run `omarchy-theme watch -- <cmd>` or watch `current/` with std's
posix inotify yourself; the debounce/swap rules are contract §4.
