const std = @import("std");
const gpu = @import("gpu");

const Renderer = @This();
const log = std.log.scoped(.oceanman);

device: *gpu.Device,
surface: *gpu.Surface,
swapchain: *gpu.SwapChain,
queue: *gpu.Queue,
pipeline: *gpu.RenderPipeline,

pub fn init(gpa: std.mem.Allocator, device: *gpu.Device, surface: *gpu.Surface) Renderer {
    log.info("initializng renderer...", .{});

    // Swapchain
    var swapchain = device.createSwapChain(surface, &gpu.SwapChain.Descriptor {
        .width = 640,
        .height = 480,
        .usage = gpu.Texture.UsageFlags {
            .render_attachment = true
        },
        .present_mode = gpu.PresentMode.fifo,
        .format = gpu.Texture.Format.bgra8_unorm
    });


    // Render pipeline
    // FIXME: this cannot be the best way to add a sentinel
    var file = std.fs.cwd().openFile("resources/shader.wgsl", .{ .mode = std.fs.File.OpenMode.read_only }) catch unreachable;
    defer file.close();
    const file_contents = file.readToEndAlloc(gpa, std.math.inf_u64) catch unreachable;
    defer gpa.free(file_contents);    
    const shader_source = gpa.alloc(u8, file_contents.len + 1) catch unreachable;
    defer gpa.free(shader_source);
    std.mem.copyForwards(u8, shader_source, file_contents);
    shader_source[shader_source.len - 1] = 0;

    var shader_module = device.createShaderModuleWGSL("shaders", shader_source[0..(shader_source.len - 1) :0]);
    defer shader_module.release();

    var pipeline = device.createRenderPipeline(&gpu.RenderPipeline.Descriptor {
        .layout = device.createPipelineLayout(&gpu.PipelineLayout.Descriptor.init(.{})),
        .vertex = gpu.VertexState.init(.{
            .module = shader_module,
            .entry_point = "vs_main"
        }),
        .fragment = &gpu.FragmentState.init(.{
            .module = shader_module,
            .entry_point = "fs_main",
            .targets = &.{
                gpu.ColorTargetState {
                    .format = .bgra8_unorm,
                    .blend = &gpu.BlendState {
                        .color = .{},
                        .alpha = .{}
                    },
                    .write_mask = gpu.ColorWriteMaskFlags.all
                }
            }
        }),
        .primitive = .{},
        .depth_stencil = null,
        .multisample = .{}
    });

    log.info("renderer initialized!", .{});
    return .{ 
        .device = device, 
        .surface = surface,
        .swapchain = swapchain,
        .queue = device.getQueue(),
        .pipeline = pipeline 
    };

}

pub fn update(this: *Renderer) void {
    var next_texture = this.swapchain.getCurrentTextureView();
    defer next_texture.release();

    var encoder = this.device.createCommandEncoder(&.{});
    defer encoder.release();
    
    var renderPass = encoder.beginRenderPass(&gpu.RenderPassDescriptor.init(.{
        .color_attachments = &.{
            gpu.RenderPassColorAttachment {
                .clear_value = gpu.Color { .r = 0.0, .g = 0.0, .b = 0.0, .a = 1.0 },
                .load_op = gpu.LoadOp.clear,
                .store_op = gpu.StoreOp.store,
                .view = next_texture
            }
        }
    }));
    defer renderPass.release();

    renderPass.setPipeline(this.pipeline);
    renderPass.draw(3, 1, 0, 0);
    renderPass.end();
        
    var commands = encoder.finish(&.{});
    defer commands.release();

    this.queue.submit(&.{commands});
    this.swapchain.present();
}

pub fn deinit(this: *Renderer) void {
    _ = this;
    log.info("deinitializing renderer", .{});
}
