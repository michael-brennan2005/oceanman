const std = @import("std");
const gpu = @import("gpu");
const zmath = @import("zmath");
const glfw = @import("glfw");

const Mat = zmath.Mat;
const Vec = zmath.Vec;

const Model = @import("loader.zig").Model;
const Camera = @import("camera.zig").Camera;

const LightingResource = @import("resources.zig").LightingResource;
const SceneResource = @import("resources.zig").SceneResource;
const MeshResource = @import("resources.zig").MeshResource;

const MeshPipeline = @This();

queue: *gpu.Queue,
pipeline: *gpu.RenderPipeline,

lighting_resource: LightingResource,
scene_resource: SceneResource,
mesh_resource: MeshResource,

fn shaderModuleFromPath(gpa: std.mem.Allocator, path: []const u8, device: *gpu.Device) !*gpu.ShaderModule {
    var file = std.fs.cwd().openFile(path, .{ .mode = std.fs.File.OpenMode.read_only }) catch unreachable;
    defer file.close();
    const file_contents = file.readToEndAlloc(gpa, std.math.inf_u64) catch unreachable;
    defer gpa.free(file_contents);    
    const shader_source = gpa.alloc(u8, file_contents.len + 1) catch unreachable;
    defer gpa.free(shader_source);
    std.mem.copyForwards(u8, shader_source, file_contents);
    shader_source[shader_source.len - 1] = 0;

    var shader_module = device.createShaderModuleWGSL("shaders", shader_source[0..(shader_source.len - 1) :0]);
    return shader_module;
}

pub fn init(gpa: std.mem.Allocator, device: *gpu.Device, queue: *gpu.Queue, lighting_resource: LightingResource, scene_resource: SceneResource) MeshPipeline {
    var mesh_resource = MeshResource.init(gpa, device, "resources/viper.m3d", true) catch unreachable;
    var shader_module = shaderModuleFromPath(gpa, "resources/mesh_pipeline.wgsl", device) catch unreachable;
    
    var pipeline = device.createRenderPipeline(&gpu.RenderPipeline.Descriptor {
        .label = "OceanMan mesh pipeline",
        .layout = device.createPipelineLayout(&gpu.PipelineLayout.Descriptor.init(.{
            .bind_group_layouts = &.{ scene_resource.bg_layout, lighting_resource.bg_layout, mesh_resource.bg_layout }
        })),
        .vertex = gpu.VertexState.init(.{
            .module = shader_module,
            .entry_point = "vs_main",
            .buffers = &.{
                mesh_resource.vertex_buffer_layout
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
        .queue = queue,
        .pipeline = pipeline,
        .lighting_resource = lighting_resource,
        .scene_resource = scene_resource,
        .mesh_resource = mesh_resource
    };
    
}

pub fn update(this: *MeshPipeline, pass: *gpu.RenderPassEncoder) void {
    pass.setPipeline(this.pipeline);
    pass.setBindGroup(0, this.scene_resource.bg, null);
    pass.setBindGroup(1, this.lighting_resource.bg, null);
    pass.setBindGroup(2, this.mesh_resource.bg, null);

    pass.setVertexBuffer(0, this.mesh_resource.vertex_buffer, 0, this.mesh_resource.vertex_buffer_count * 8 * @sizeOf(f32));
    pass.draw(@intCast(u32,this.mesh_resource.vertex_buffer_count), 1, 0, 0);
}
pub fn deinit(this: *MeshPipeline) void {
    _ = this;
    // TODO
}