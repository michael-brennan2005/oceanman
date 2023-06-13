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

const MeshPipeline = @This();

queue: *gpu.Queue,
pipeline: *gpu.RenderPipeline,

uniforms: Uniforms,
uniform_buffer: *gpu.Buffer,
uniform_binding: *gpu.BindGroup,

vertex_buffer: *gpu.Buffer,
vertex_count: usize,

lighting_resource: LightingResource,
scene_resource: SceneResource,

// TODO: make this an object that actually manages buffer, can create a bind group and what not
const Uniforms = struct {
    perspective: Mat,
    view: Mat,
    model: Mat,
    padding: [12]f32 = [_]f32{0.0} ** 12,
    camera_pos: Vec,

    pub fn write_to_buffer(this: *Uniforms, queue: *gpu.Queue, buffer: *gpu.Buffer) void {
        var uniforms_slice: []Uniforms = undefined;
        uniforms_slice.len = 1;
        uniforms_slice.ptr = @ptrCast([*]Uniforms, this);
        queue.writeBuffer(buffer, 0, uniforms_slice);
    }
};

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
    var model = Model.createFromFile(gpa, "resources/viper.m3d", true) catch unreachable;
    
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
    // Write uniform buffers and binding group.
    var uniforms = Uniforms {
        .model = zmath.rotationY(std.math.pi),
        .view = zmath.identity(),
        .perspective = zmath.identity(),
        .camera_pos = zmath.f32x4(0.0, 0.0, 0.0, 1.0)
    };
    var uniform_buffer = device.createBuffer(&.{
        .label = "Uniform buffer",
        .usage = gpu.Buffer.UsageFlags {
            .uniform = true,
            .copy_dst = true
        },
        .size = @sizeOf(Uniforms)
    });

    uniforms.write_to_buffer(queue, uniform_buffer);

    // Write texture
    var texture = device.createTexture(&gpu.Texture.Descriptor.init(.{
        .label = "Model texture",
        .usage = gpu.Texture.UsageFlags {
            .texture_binding = true,
            .copy_dst = true
        },
        .format = gpu.Texture.Format.rgba8_unorm,
        .size = gpu.Extent3D {
            .depth_or_array_layers = 1,
            .height = model.texture_height,
            .width = model.texture_width
        }
    }));
    queue.writeTexture(
        &gpu.ImageCopyTexture {
            .aspect = gpu.Texture.Aspect.all,
            .mip_level = 0,
            .origin = gpu.Origin3D {
                .x = 0,
                .y = 0,
                .z = 0
            },
            .texture = texture
        },
        &gpu.Texture.DataLayout {
            .bytes_per_row = 4 * model.texture_width,
            .rows_per_image = model.texture_height,
            .offset = 0,
        },
        &gpu.Extent3D {
            .depth_or_array_layers = 1,
            .width = model.texture_width,
            .height = model.texture_height,
        },
        model.texture.?
    );

    var uniform_layout = device.createBindGroupLayout(&gpu.BindGroupLayout.Descriptor.init(.{
        .entries = &.{
            gpu.BindGroupLayout.Entry.buffer(
                0, 
                gpu.ShaderStageFlags {
                    .vertex = true,
                    .fragment = true
                },
                gpu.Buffer.BindingType.uniform,
                false,
                @sizeOf(Uniforms)),
            gpu.BindGroupLayout.Entry.texture(
                1,
                gpu.ShaderStageFlags {
                    .fragment = true
                },
                gpu.Texture.SampleType.float,
                gpu.TextureView.Dimension.dimension_2d,
                false
            )
        }
    }));
    var uniform_binding = device.createBindGroup(&gpu.BindGroup.Descriptor.init(.{
        .layout = uniform_layout,
        .entries = &.{
            gpu.BindGroup.Entry.buffer(0, uniform_buffer, 0, @sizeOf(Uniforms)),
            gpu.BindGroup.Entry.textureView(1, texture.createView(&gpu.TextureView.Descriptor {
                .aspect = gpu.Texture.Aspect.all,
                .base_array_layer = 0,
                .array_layer_count = 1,
                .base_mip_level = 0,
                .mip_level_count = 1,
                .dimension = gpu.TextureView.Dimension.dimension_2d,
                .format = gpu.Texture.Format.rgba8_unorm
            }))
        }
    }));

    var shader_module = shaderModuleFromPath(gpa, "resources/shader.wgsl", device) catch unreachable;
    
    var pipeline = device.createRenderPipeline(&gpu.RenderPipeline.Descriptor {
        .label = "OceanMan mesh pipeline",
        .layout = device.createPipelineLayout(&gpu.PipelineLayout.Descriptor.init(.{
            .bind_group_layouts = &.{ uniform_layout, lighting_resource.bg_layout, scene_resource.bg_layout }
        })),
        .vertex = gpu.VertexState.init(.{
            .module = shader_module,
            .entry_point = "vs_main",
            .buffers = &.{
                gpu.VertexBufferLayout.init(.{
                    .array_stride = 8 * @sizeOf(f32),
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
                        },
                        gpu.VertexAttribute {
                            .format = gpu.VertexFormat.float32x2,
                            .offset = 6 * @sizeOf(f32),
                            .shader_location = 2
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
        .uniforms = uniforms,
        .uniform_buffer = uniform_buffer,
        .uniform_binding = uniform_binding,
        .vertex_buffer = vertex_buffer,
        .vertex_count = model.buffer.len / 8,
        .lighting_resource = lighting_resource,
        .scene_resource = scene_resource
    };
    
}

pub fn update(this: *MeshPipeline, pass: *gpu.RenderPassEncoder, camera: *Camera, ratio: f32) void {
    this.uniforms.perspective = zmath.perspectiveFovLh(1.22, ratio, 0.01, 100.0);
    this.uniforms.view = zmath.lookAtLh(camera.position, camera.position + camera.front, camera.up);
    this.uniforms.camera_pos = camera.position;

    this.uniforms.write_to_buffer(this.queue, this.uniform_buffer);

    pass.setPipeline(this.pipeline);
    pass.setBindGroup(0, this.uniform_binding, null);
    pass.setBindGroup(1, this.lighting_resource.bg, null);
    pass.setBindGroup(2, this.scene_resource.bg, null);
    pass.setVertexBuffer(0, this.vertex_buffer, 0, this.vertex_count * 8 * @sizeOf(f32));
    pass.draw(@intCast(u32,this.vertex_count), 1, 0, 0);
}
pub fn deinit(this: *MeshPipeline) void {
    _ = this;
    // TODO
}