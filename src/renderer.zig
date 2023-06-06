const std = @import("std");
const gpu = @import("gpu");

const Object = @import("obj_loader.zig").Object;

const Renderer = @This();
const log = std.log.scoped(.oceanman);

device: *gpu.Device,
surface: *gpu.Surface,
swapchain: *gpu.SwapChain,
queue: *gpu.Queue,
pipeline: *gpu.RenderPipeline,

vertex_buffer: *gpu.Buffer,
vertex_count: usize,
index_buffer: *gpu.Buffer,
index_count: usize,

pub fn init(gpa: std.mem.Allocator, device: *gpu.Device, surface: *gpu.Surface) Renderer {
    log.info("initializng renderer...", .{});
    
    const queue = device.getQueue();

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

    var model = Object.createFromFile(gpa, "resources/triangle.obj");
    _ = model;
    
    // Write vertex and index buffers
    var vertex_buffer = device.createBuffer(&.{
        .usage = gpu.Buffer.UsageFlags {
            .vertex = true,
            .copy_dst = true
        },
        .size = 24 * @sizeOf(f32)
    });
    var vertex_count: usize = 8;
    var slice = [_]f32{
        0.999999, -0.999999, -0.999999,
        0.999999, -0.999999, 0.999999,
        -0.999999, -0.999999, 0.999999,
        -0.999999, -0.999999, -0.999999,
        0.999999, 0.999999, -0.999999,
        0.999999, 0.999999, 0.999999,
        -0.999999, 0.999999, 0.999999,
        -0.999999, 0.999999, -0.999999
    };
    queue.writeBuffer(vertex_buffer, 0, &slice);

    var index_buffer = device.createBuffer(&.{
        .usage = gpu.Buffer.UsageFlags {
            .index = true,
            .copy_dst = true
        },
        .size = 36 * @sizeOf(u32)
    });
    var index_count: usize = 36;
    var index_slice = [_]u32{
        1, 2, 3,
        7, 6, 5,
        4, 5, 1,
        5, 6, 2,
        2, 6, 7,
        0, 3, 7,
        0, 1, 3,
        4, 7, 5,
        0, 4, 1,
        1, 5, 2,
        3, 2, 7,
        4, 0, 7,
    };
    queue.writeBuffer(index_buffer, 0, &index_slice);

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
            .entry_point = "vs_main",
            .buffers = &.{
                gpu.VertexBufferLayout.init(.{
                    .array_stride = 3 * @sizeOf(f32),
                    .attributes = &.{
                        gpu.VertexAttribute {
                            .format = gpu.VertexFormat.float32x3,
                            .offset = 0,
                            .shader_location = 0
                        }
                    }
                })
            }
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
        .queue = queue,
        .pipeline = pipeline,
        .vertex_buffer = vertex_buffer,
        .vertex_count = vertex_count,
        .index_buffer = index_buffer,
        .index_count = index_count
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
        },
    }));
    defer renderPass.release();

    renderPass.setPipeline(this.pipeline);
    renderPass.setVertexBuffer(0, this.vertex_buffer, 0, this.vertex_count * 3 * @sizeOf(f32));
    renderPass.setIndexBuffer(this.index_buffer, gpu.IndexFormat.uint32, 0, this.index_count * @sizeOf(u32));

    renderPass.drawIndexed(36, 1, 0, 0, 0);
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
