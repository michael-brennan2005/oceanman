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
    mesh_resource: *MeshResource,

    pub fn init(gpa: std.mem.Allocator, device: *gpu.Device, queue: *gpu.Queue,  scene_resource: *SceneResource, lighting_resource: *LightingResource, mesh_resource: *MeshResource) MeshPipeline {
        var shader_module = ShaderResource.init(gpa, device, "resources/mesh_pipeline.wgsl");
        
        var pipeline = device.createRenderPipeline(&gpu.RenderPipeline.Descriptor {
            .label = "OceanMan mesh pipeline",
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
};

pub const LightingPipeline = struct {
    device: *gpu.Device,
    queue: *gpu.Queue,
    pipeline: *gpu.RenderPipeline,

    lighting_resource: *LightingResource,
    scene_resource: *SceneResource,
    cube_resource: *UntexturedMeshResource,

    pub fn init(gpa: std.mem.Allocator, device: *gpu.Device, queue: *gpu.Queue, scene_resource: *SceneResource, lighting_resource: *LightingResource) LightingPipeline {
        var cube_resource = gpa.create(UntexturedMeshResource) catch unreachable;
        cube_resource.* = UntexturedMeshResource.init(gpa, device, "resources/cube.m3d") catch unreachable;
        var shader_module = ShaderResource.init(gpa, device, "resources/lighting_pipeline.wgsl");
        
        var pipeline = device.createRenderPipeline(&gpu.RenderPipeline.Descriptor {
            .label = "OceanMan lighting pipeline",
            .layout = device.createPipelineLayout(&gpu.PipelineLayout.Descriptor.init(.{
                .bind_group_layouts = &.{ scene_resource.bg_layout, lighting_resource.bg_layout, cube_resource.bg_layout }
            })),
            .vertex = gpu.VertexState.init(.{
                .module = shader_module.module,
                .entry_point = "vs_main",
                .buffers = &.{
                    cube_resource.vertex_buffer_layout
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
            .lighting_resource = lighting_resource,
            .scene_resource = scene_resource,
            .cube_resource = cube_resource
        };
        
    }

    pub fn update(this: *LightingPipeline, pass: *gpu.RenderPassEncoder) void {
        pass.setPipeline(this.pipeline);
        pass.setBindGroup(0, this.scene_resource.bg, null);
        pass.setBindGroup(1, this.lighting_resource.bg, null);
        pass.setBindGroup(2, this.cube_resource.bg, null);

        pass.setVertexBuffer(0, this.cube_resource.vertex_buffer, 0, this.cube_resource.vertex_buffer_count * 6 * @sizeOf(f32));
        pass.draw(@intCast(u32,this.cube_resource.vertex_buffer_count), 1, 0, 0);
    }

    pub fn deinit(this: *LightingPipeline) void {
        _ = this;
        // TODO
    }
};