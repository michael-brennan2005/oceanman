const std = @import("std");
const glfw = @import("glfw");
const gpu = @import("gpu");

const Renderer = @import("renderer.zig");

// MARK: Loggging
pub const std_options = struct {
    pub const log_level = .info;
    pub const logFn = log;
};

fn log(
    comptime level: std.log.Level,
    comptime scope: @TypeOf(.EnumLiteral),
    comptime format: []const u8,
    args: anytype,
) void {
    const scope_prefix = "(" ++ @tagName(scope) ++ "): ";
    const prefix = "[" ++ comptime level.asText() ++ "] " ++ scope_prefix;

    std.debug.getStderrMutex().lock();
    defer std.debug.getStderrMutex().unlock();
    const stderr = std.io.getStdErr().writer();
    nosuspend stderr.print(prefix ++ format ++ "\n", args) catch return;
}

const glfw_log = std.log.scoped(.glfw);
const wgpu_log = std.log.scoped(.webgpu);

// MARK: GPU utilities
pub const GPUInterface = gpu.dawn.Interface;

pub fn detectGLFWOptions() glfw.BackendOptions {
    const target = @import("builtin").target;
    if (target.isDarwin()) return .{ .cocoa = true };
    return switch (target.os.tag) {
        .windows => .{ .win32 = true },
        .linux => .{ .x11 = true, .wayland = true },
        else => .{},
    };
}

pub fn createSurfaceForWindow(
    instance: *gpu.Instance,
    window: glfw.Window,
    comptime glfw_options: glfw.BackendOptions,
) *gpu.Surface {
    const glfw_native = glfw.Native(glfw_options);
    const extension = if (glfw_options.win32) gpu.Surface.Descriptor.NextInChain{
        .from_windows_hwnd = &.{
            .hinstance = std.os.windows.kernel32.GetModuleHandleW(null).?,
            .hwnd = glfw_native.getWin32Window(window),
        },
    } else if (glfw_options.x11) gpu.Surface.Descriptor.NextInChain{
        .from_xlib_window = &.{
            .display = glfw_native.getX11Display(),
            .window = glfw_native.getX11Window(window),
        },
    } else if (glfw_options.cocoa) {
        @panic("Cocoa not supported.");
    } else if (glfw_options.wayland) {
        @panic("Wayland not supported");
    } else unreachable;

    return instance.createSurface(&gpu.Surface.Descriptor{
        .next_in_chain = extension,
    });
}

const RequestAdapterResponse = struct {
    status: gpu.RequestAdapterStatus,
    adapter: *gpu.Adapter,
    message: ?[*:0]const u8,
};

inline fn requestAdapterCallback(
    context: *?RequestAdapterResponse,
    status: gpu.RequestAdapterStatus,
    adapter: *gpu.Adapter,
    message: ?[*:0]const u8,
) void {
    if (status != .success) {
        wgpu_log.err("Error requesting adapter: {?s}", .{message});
        std.process.exit(1);
    }

    wgpu_log.info("Adapter successfully requested.", .{});
    context.* = RequestAdapterResponse{
        .status = status,
        .adapter = adapter,
        .message = message,
    };
}
fn glfwErrorCallback(error_code: glfw.ErrorCode, description: [:0]const u8) void {
    std.log.err("glfw: {}: {s}\n", .{ error_code, description });
}

// MARK: Main
pub fn main() !void {
    gpu.Impl.init();

    glfw_log.info("initializing GLFW...", .{});
    glfw.setErrorCallback(glfwErrorCallback);
    if (!glfw.init(.{})) {
        glfw_log.err("Failed to initalize GLFW: {?s}", .{glfw.getErrorString()});
        std.process.exit(1);
    }
    defer glfw.terminate();
    glfw_log.info("GLFW initialized!", .{});

    const window = glfw.Window.create(640, 480, "OceanMan", null, null, .{}) orelse {
        glfw_log.err("Failed to create window: {?s}", .{glfw.getErrorString()});
        std.process.exit(1);
    };
    defer window.destroy();
    defer glfw_log.info("destroying window", .{});

    const instance = gpu.createInstance(&.{}) orelse {
        wgpu_log.err("Failed to create instance.", .{});
        std.process.exit(1);
    };

    var response: ?RequestAdapterResponse = null;
    instance.requestAdapter(&.{}, &response, requestAdapterCallback);

    if (response == null) {
        wgpu_log.err("RequestAdapterResponse found null", .{});
        std.process.exit(1);
    }

    var limits: gpu.SupportedLimits = .{};
    _ = response.?.adapter.getLimits(&limits);
    wgpu_log.info("{?}", .{limits.limits});

    var device: ?*gpu.Device = response.?.adapter.createDevice(null);
    if (device == null) {
        wgpu_log.err("Failed to create device.", .{});
        std.process.exit(1);
    }

    var surface = createSurfaceForWindow(instance, window, comptime detectGLFWOptions());

    var renderer: Renderer = Renderer.init(device.?, surface);
    defer renderer.deinit();

    while (!window.shouldClose()) {
        glfw.pollEvents();

        renderer.update();
    }
}
