const std = @import("std");

const glfw = @import("libs/mach-glfw/build.zig");
const gpu_dawn_sdk = @import("libs/mach-gpu-dawn/sdk.zig");
const gpu_sdk = @import("libs/mach-gpu/sdk.zig");
const system_sdk = @import("libs/mach-glfw/system_sdk.zig");
const zmath = @import("libs/zmath/build.zig");

// Although this function looks imperative, note that its job is to
// declaratively construct a build graph that will be executed by an external
// runner.
pub fn build(b: *std.Build) !void {
    const target = b.standardTargetOptions(.{});
    const optimize = b.standardOptimizeOption(.{});

    const exe = b.addExecutable(.{
        .name = "oceanman",
        .root_source_file = .{ .path = "src/main.zig" },
        .target = target,
        .optimize = optimize,
    });

    exe.addModule("glfw", glfw.module(b));
    try glfw.link(b, exe, .{});

    const gpu_dawn = gpu_dawn_sdk.Sdk(.{ .glfw_include_dir = "libs/mach-glfw/upstream/glfw/include", .system_sdk = system_sdk });
    const gpu_dawn_options = gpu_dawn.Options{ .from_source = false, .debug = false, .separate_libs = false };
    const gpu = gpu_sdk.Sdk(.{ .gpu_dawn = gpu_dawn });

    exe.addModule("gpu", gpu.module(b));
    try gpu.link(b, exe, .{ .gpu_dawn_options = gpu_dawn_options });

    const zmath_module = b.createModule(.{
        .source_file = .{ .path = "libs/zmath/src/zmath.zig" },
        .dependencies = &.{},
    });
    exe.addModule("zmath", zmath_module);
    
    b.installArtifact(exe);

    const run_cmd = b.addRunArtifact(exe);
    run_cmd.step.dependOn(b.getInstallStep());
    if (b.args) |args| {
        run_cmd.addArgs(args);
    }
    const run_step = b.step("run", "Run the app");
    run_step.dependOn(&run_cmd.step);

    const unit_tests = b.addTest(.{
        .root_source_file = .{ .path = "src/main.zig" },
        .target = target,
        .optimize = optimize,
    });
    const run_unit_tests = b.addRunArtifact(unit_tests);
    const test_step = b.step("test", "Run unit tests");
    test_step.dependOn(&run_unit_tests.step);
}
