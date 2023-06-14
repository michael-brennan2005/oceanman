const std = @import("std");
const gpu = @import("gpu");
const zmath = @import("zmath");
const glfw = @import("glfw");

const Mat = zmath.Mat;
const Vec = zmath.Vec;

const Model = @import("loader.zig").Model;
const Camera = @import("camera.zig").Camera;

const LightingPipeline = @This();
const LightingResource = @import("resources.zig").LightingResource;
const SceneResource = @import("resources.zig").SceneResource;
const ShaderResource = @import("resources.zig").ShaderResource;

device: *gpu.Device,
queue: *gpu.Queue,
pipeline: *gpu.RenderPipeline,

// For bespoke cube mesh (TODO: get this to be an actual mesh resource this is ridiculous)
vertex_buffer: *gpu.Buffer,
vertex_count: usize,

lighting_resource: *LightingResource,
scene_resource: *SceneResource,

pub fn init(gpa: std.mem.Allocator, device: *gpu.Device, queue: *gpu.Queue, scene_resource: *SceneResource, lighting_resource: *LightingResource,) LightingPipeline {
    var model = Model.createFromFile(gpa, "resources/cube.m3d", false) catch unreachable;
    var shader_module = ShaderResource.init(gpa, device, "resources/lighting_pipeline.wgsl");
    // Write vertex and index buffers
    var vertex_buffer = device.createBuffer(&.{
        .label = "Vertex buffer",
        .usage = gpu.Buffer.UsageFlags {
            .vertex = true,
            .copy_dst = true
        },
        .size = model.buffer.len * @sizeOf(f32)
    });
    queue.writeBuffer(vertex_buffer, 0, model.buffer);
   
    var pipeline = device.createRenderPipeline(&gpu.RenderPipeline.Descriptor {
        .label = "OceanMan lighting pipeline",
        .layout = device.createPipelineLayout(&gpu.PipelineLayout.Descriptor.init(.{
            .bind_group_layouts = &.{ scene_resource.bg_layout, lighting_resource.bg_layout }
        })),
        .vertex = gpu.VertexState.init(.{
            .module = shader_module.module,
            .entry_point = "vs_main",
            .buffers = &.{
                gpu.VertexBufferLayout.init(.{
                    .array_stride = 6 * @sizeOf(f32),
                    .attributes = &.{
                        gpu.VertexAttribute {
                            .format = gpu.VertexFormat.float32x3,
                            .offset = 0,
                            .shader_location = 0
                        },
                        gpu.VertexAttribute {
                            .format = gpu.VertexFormat.float32x3,
                            .offset = 3 * @sizeOf(f32),
                            .shader_location = 1
                        }
                    }
                })
            }
        }),
        .fragment = &gpu.FragmentState.init(.{
            .module = shader_module.module,
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
        .depth_stencil = &.{
            .format = gpu.Texture.Format.depth24_plus,
            .depth_compare = gpu.CompareFunction.less,
            .depth_write_enabled = true,
            .stencil_read_mask = 0,
            .stencil_write_mask = 0
        },
        .multisample = .{}
    });
    
    return .{
        .device = device,
        .queue = queue,
        .pipeline = pipeline,
        .vertex_buffer = vertex_buffer,
        .vertex_count = model.buffer.len / 6,
        .lighting_resource = lighting_resource,
        .scene_resource = scene_resource,
    };
    
}

pub fn update(this: *LightingPipeline, pass: *gpu.RenderPassEncoder) void {
    pass.setPipeline(this.pipeline);
    pass.setBindGroup(0, this.scene_resource.bg, null);
    pass.setBindGroup(1, this.lighting_resource.bg, null);

    pass.setVertexBuffer(0, this.vertex_buffer, 0, this.vertex_count * 6 * @sizeOf(f32));
    pass.draw(@intCast(u32,this.vertex_count), 1, 0, 0);
}

pub fn deinit(this: *LightingPipeline) void {
    _ = this;
    // TODO
}