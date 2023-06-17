const std = @import("std");
const gpu = @import("gpu");
const zmath = @import("zmath");
const glfw = @import("glfw");

const Mat = zmath.Mat;
const Vec = zmath.Vec;

const Camera = @import("camera.zig").Camera;

const LightingResource = @import("resources.zig").LightingResource;
const SceneResource = @import("resources.zig").SceneResource;
const MeshResource = @import("resources.zig").MeshResource;
const ShaderResource = @import("resources.zig").ShaderResource;
const UntexturedMeshResource = @import("resources.zig").UntexturedMeshResource;

pub const MeshPipeline = struct {
    queue: *gpu.Queue,
    pipeline: *gpu.RenderPipeline,

    lighting_resource: *LightingResource,
    scene_resource: *SceneResource,
    mesh_resources: []*MeshResource,

    pub fn init(gpa: std.mem.Allocator, device: *gpu.Device, queue: *gpu.Queue,  scene_resource: *SceneResource, lighting_resource: *LightingResource, mesh_resources: []*MeshResource) MeshPipeline {
        var shader_module = ShaderResource.init(gpa, device, "resources/mesh_pipeline.wgsl");
        
        var pipeline = device.createRenderPipeline(&gpu.RenderPipeline.Descriptor {
            .label = "OceanMan mesh pipeline",
            .layout = device.createPipelineLayout(&gpu.PipelineLayout.Descriptor.init(.{
                .bind_group_layouts = &.{ scene_resource.bg_layout, lighting_resource.bg_layout, mesh_resources[0].bg_layout }
            })),
            .vertex = gpu.VertexState.init(.{
                .module = shader_module.module,
                .entry_point = "vs_main",
                .buffers = &.{
                    mesh_resources[0].vertex_buffer_layout
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
            .queue = queue,
            .pipeline = pipeline,
            .lighting_resource = lighting_resource,
            .scene_resource = scene_resource,
            .mesh_resources = mesh_resources
        };
        
    }

    pub fn update(this: *MeshPipeline, pass: *gpu.RenderPassEncoder) void {
        pass.setPipeline(this.pipeline);
        pass.setBindGroup(0, this.scene_resource.bg, null);
        pass.setBindGroup(1, this.lighting_resource.bg, null);

        for (this.mesh_resources) |mesh_resource| {
             pass.setBindGroup(2, mesh_resource.bg, null);
            pass.setVertexBuffer(0, mesh_resource.vertex_buffer, 0, mesh_resource.vertex_buffer_count * 8 * @sizeOf(f32));
            pass.draw(@intCast(u32,mesh_resource.vertex_buffer_count), 1, 0, 0);
        }
       
    }
    pub fn deinit(this: *MeshPipeline) void {
        _ = this;
        // TODO
    }
};

pub fn shadowMapPipeline(gpa: std.mem.Allocator, device: *gpu.Device, scene_resource: *SceneResource, lighting_resource: *LightingResource, mesh_resource: *MeshResource) *gpu.RenderPipeline {
    var shader_module = ShaderResource.init(gpa, device, "resources/shadowmap.wgsl");
        
    return device.createRenderPipeline(&gpu.RenderPipeline.Descriptor {
        .label = "OceanMan shadow-map pipeline",
        .layout = device.createPipelineLayout(&gpu.PipelineLayout.Descriptor.init(.{
            .bind_group_layouts = &.{ scene_resource.bg_layout, lighting_resource.bg_layout, mesh_resource.bg_layout }
        })),
        .vertex = gpu.VertexState.init(.{
            .module = shader_module.module,
            .entry_point = "vs_main",
            .buffers = &.{
                mesh_resource.vertex_buffer_layout
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
}