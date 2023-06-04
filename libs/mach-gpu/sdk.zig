const std = @import("std");

pub fn Sdk(comptime deps: anytype) type {
    return struct {
        pub fn testStep(b: *std.Build, optimize: std.builtin.OptimizeMode, target: std.zig.CrossTarget, options: Options) !*std.build.RunStep {
            const main_tests = b.addTest(.{
                .name = "gpu-tests",
                .root_source_file = .{ .path = sdkPath("/src/main.zig") },
                .target = target,
                .optimize = optimize,
            });
            try link(b, main_tests, options);
            b.installArtifact(main_tests);
            return b.addRunArtifact(main_tests);
        }

        pub const Options = struct {
            gpu_dawn_options: deps.gpu_dawn.Options = .{},
        };

        var _module: ?*std.build.Module = null;

        pub fn module(b: *std.Build) *std.build.Module {
            if (_module) |m| return m;
            _module = b.createModule(.{
                .source_file = .{ .path = sdkPath("/src/main.zig") },
            });
            return _module.?;
        }

        pub fn link(b: *std.Build, step: *std.build.CompileStep, options: Options) !void {
            if (step.target.toTarget().cpu.arch != .wasm32) {
                try deps.gpu_dawn.link(b, step, options.gpu_dawn_options);
                step.addCSourceFile(sdkPath("/src/mach_dawn.cpp"), &.{"-std=c++17"});
                step.addIncludePath(sdkPath("/src"));
            }
        }

        fn sdkPath(comptime suffix: []const u8) []const u8 {
            if (suffix[0] != '/') @compileError("suffix must be an absolute path");
            return comptime blk: {
                const root_dir = std.fs.path.dirname(@src().file) orelse ".";
                break :blk root_dir ++ suffix;
            };
        }
    };
}
