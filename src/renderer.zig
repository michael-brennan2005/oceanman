const std = @import("std");
const gpu = @import("gpu");

const Renderer = @This();
const log = std.log.scoped(.webgpu);

device: *gpu.Device,
surface: *gpu.Surface,
swapchain: *gpu.SwapChain,
queue: *gpu.Queue,
pipeline: *gpu.RenderPipeline,

pub fn init(device: *gpu.Device, surface: *gpu.Surface) Renderer {
    log.info("initializng renderer...", .{});
    log.info("renderer initialized!", .{});

    // Swapchain
    var swapchain = device.createSwapChain(surface, &gpu.SwapChain.Descriptor {
        .width = 640,
        .height = 480,
        .usage = gpu.Texture.UsageFlags {
            .render_attachment = true
        },
        .present_mode = gpu.PresentMode.fifo,
        .format = gpu.Texture.Format.rgba8_unorm
    });

    // Render pipeline
    var pipeline = device.createRenderPipeline(gpu.RenderPipeline.Descriptor {
        .layout = device.createPipelineLayout(&gpu.PipelineLayout.Descriptor.init(.{})),
        .vertex = gpu.VertexState.init(.{}), // TODO: initialize w/ module and entry point
        .fragment = gpu.FragmentState.init(.{}), // TODO: initialize w/ module and entry point
        .primitive = .{},
        .depth_stencil = .{},
        .multisample = .{}
    });

    return .{ 
        .device = device, 
        .surface = surface,
        .swapchain = swapchain,
        .queue = device.getQueue(),
        .pipeline = pipeline 
    };

}

pub fn update(this: Renderer) void {
    _ = this;
}

pub fn deinit(this: Renderer) void {
    _ = this;
    log.info("deinitializing renderer", .{});
}
