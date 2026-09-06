const std = @import("std");

pub fn build(b: *std.Build) void {
    const target = b.standardTargetOptions(.{});
    const optimize = b.standardOptimizeOption(.{});

    const zgui_dep = b.dependency("zgui", .{
        .target = target,
        .optimize = optimize,
    });

    const mod = b.createModule(.{
        .root_source_file = b.path("src/main.zig"),
        .target = target,
        .optimize = optimize,
    });
    mod.addImport("zgui", zgui_dep.module("root"));
    mod.addImport("omarchy_theme", b.createModule(.{
        .root_source_file = b.path("../../bindings/zig/omarchy_theme.zig"),
        .target = target,
        .optimize = optimize,
    }));

    // omarchy C ABI shared lib built by cargo at ../../target/release
    mod.addLibraryPath(b.path("../../target/release"));
    mod.linkSystemLibrary("omarchy_theme_abi", .{});
    mod.addRPath(.{ .cwd_relative = b.pathFromRoot("../../target/release") });
    mod.link_libc = true;
    mod.linkLibrary(zgui_dep.artifact("imgui"));

    const exe = b.addExecutable(.{
        .name = "zgui-verify",
        .root_module = mod,
    });
    // zig 0.16 self-hosted linker cannot handle glibc crt1.o .sframe
    // relocations (upstream ziglang/zig#31272); LLVM+LLD can. Drop this
    // once zig >= 0.17 is the floor (0.17 also removed std.meta.fields,
    // which zgui still uses).
    exe.use_llvm = true;
    exe.use_lld = true;

    b.installArtifact(exe);

    const run_cmd = b.addRunArtifact(exe);
    run_cmd.step.dependOn(b.getInstallStep());
    if (b.args) |args| run_cmd.addArgs(args);
    b.step("run", "Run the zgui verification").dependOn(&run_cmd.step);
}
