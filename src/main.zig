const std = @import("std");
const glfw = @import("glfw");
const gpu = @import("gpu");
const zmath = @import("zmath");

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
const app_log = std.log.scoped(.oceanman);

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

inline fn deviceLoggingCallback(
    context: *usize,
    typ: gpu.LoggingType, 
    message:[*:0]const u8
) void {
    _ = context;
    switch (typ) {
        .verbose => wgpu_log.debug("{s}", .{message}),
        .info => wgpu_log.info("{s}", .{message}),
        .warning => wgpu_log.warn("{s}", .{message}),
        .err => wgpu_log.err("{s}", .{message})
    }
}

inline fn deviceErrorCallback(
    context: *usize,
    typ: gpu.ErrorType,
    message: [*:0]const u8
) void {
    _ = context;
    wgpu_log.err("{s} ({?})", .{message, typ});
    std.process.exit(1);
}

// MARK: Main
var renderer_handle: ?*Renderer = null;

pub fn glfwKeyCallback(window: glfw.Window, key: glfw.Key, scancode: i32, action: glfw.Action, mods: glfw.Mods) void {
    _ = mods;
    _ = scancode;
    _ = window;
    if (renderer_handle) |renderer| {
        if (action == glfw.Action.press) {
            renderer.onKeyDown(key);
        } else if (action == glfw.Action.release) {
            renderer.onKeyUp(key);
        }
    }
}

pub fn glfwMouseCallback(window: glfw.Window, button: glfw.MouseButton, action: glfw.Action, mods: glfw.Mods) void {
    _ = mods;
    if (renderer_handle) |renderer| {
        if (action == glfw.Action.press) {
            // FIXME: code smell having to pass in window
            renderer.onMouseButtonDown(&window, button);
        } else {
            renderer.onMouseButtonUp(&window, button);
        }
    }
}

pub fn glfwCursorCallback(window: glfw.Window, xpos: f64, ypos: f64) void {
    _ = window;
    if (renderer_handle) |renderer| {
        renderer.onMouseMove(@floatCast(f32, xpos), @floatCast(f32, ypos));
    }
}

pub fn glfwResizeCallback(window: glfw.Window, width: u32, height: u32) void {
    _ = window;
    if (renderer_handle) |renderer| {
        renderer.onWindowResize(width, height);
    }
}

pub fn main() !void {
    var gpa = std.heap.GeneralPurposeAllocator(.{}){};
    const allocator = gpa.allocator();

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

    defer response.?.adapter.release();

    // MARK: where we'll set limits
    var limits: gpu.SupportedLimits = .{};
    _ = response.?.adapter.getLimits(&limits);
    wgpu_log.info("{?}", .{limits.limits});

    var device: ?*gpu.Device = response.?.adapter.createDevice(null);

    if (device == null) {
        wgpu_log.err("Failed to create device.", .{});
        std.process.exit(1);
    }

    var num: usize = 0;
    device.?.setLoggingCallback(&num, deviceLoggingCallback);
    device.?.setUncapturedErrorCallback(&num, deviceErrorCallback);

    var surface = createSurfaceForWindow(instance, window, comptime detectGLFWOptions());

    var args = std.process.argsAlloc(gpa.allocator()) catch unreachable;
    if (args.len <= 1) {
        app_log.err("need one argument for file to load", .{});
        std.process.exit(1);
    }

    var file = args[1];
    
    var renderer: Renderer = Renderer.init(allocator, device.?, surface, file);
    renderer_handle = &renderer;
    defer renderer.deinit();

    window.setKeyCallback(glfwKeyCallback);
    window.setMouseButtonCallback(glfwMouseCallback);
    window.setCursorPosCallback(glfwCursorCallback);
    window.setFramebufferSizeCallback(glfwResizeCallback);
    
    var delta_time: f32 = 0.0;
    var last_frame: f32 = @floatCast(f32, glfw.getTime());
    
    while (!window.shouldClose()) {
        glfw.pollEvents();

        renderer.update(delta_time);
        const current_time = @floatCast(f32, glfw.getTime());
        delta_time = current_time - last_frame;
        last_frame = current_time;

    }
}
