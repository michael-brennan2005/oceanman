const std = @import("std");
const gpu = @import("gpu");
const zmath = @import("zmath");
const model3d = @import("model3d");
const glfw = @import("glfw");

const Mat = zmath.Mat;
const Vec = zmath.Vec;

pub const SceneResource = struct {
    pub const Payload = extern struct {
        perspective: Mat = zmath.identity(),
        view: Mat = zmath.identity(),
        camera_pos: Vec = zmath.f32x4(0.0, 0.0, 0.0, 0.0)
    };

    payload: Payload = .{},
    buffer: *gpu.Buffer = undefined,
    bg_layout: *gpu.BindGroupLayout = undefined,
    bg: *gpu.BindGroup = undefined,

    pub fn init(device: *gpu.Device) SceneResource {
        const queue = device.getQueue();
        var resource: SceneResource = .{};
            
        // Uniform buffer
        resource.buffer = device.createBuffer(&gpu.Buffer.Descriptor {
            .label = "SceneResource - uniform buffer",
            .mapped_at_creation = false,
            .size = @sizeOf(Payload),
            .usage = gpu.Buffer.UsageFlags {
                .copy_dst = true,
                .uniform = true
            }
        });

        // TODO: get this to load in from somewhere
        resource.payload = .{};
      
        var payload_slice: []Payload = undefined;
        payload_slice.len = 1;
        payload_slice.ptr = @ptrCast([*]Payload, &resource.payload);
        queue.writeBuffer(resource.buffer, 0, payload_slice);

        resource.bg_layout = device.createBindGroupLayout(&gpu.BindGroupLayout.Descriptor.init(.{
            .entries = &.{
                gpu.BindGroupLayout.Entry.buffer(
                    0,
                    gpu.ShaderStageFlags {
                        .vertex = true,
                        .fragment = true
                    },
                    gpu.Buffer.BindingType.uniform,
                    false,
                    @sizeOf(Payload)
                )
            }
        }));

        resource.bg = device.createBindGroup(&gpu.BindGroup.Descriptor.init(.{
            .layout = resource.bg_layout,
            .entries = &.{
                gpu.BindGroup.Entry.buffer(0, resource.buffer, 0, @sizeOf(Payload))
            }
        }));

        return resource;
    }

    pub fn update(this: *SceneResource, device: *gpu.Device, ratio: f32, eyepos: Vec, focuspos: Vec, updir: Vec) void {
        this.payload.perspective = zmath.perspectiveFovLh(1.22, ratio, 0.01, 100.0);
        this.payload.view = zmath.lookAtLh(eyepos, focuspos, updir);
        this.payload.camera_pos = eyepos;
        
        var payload_slice: []Payload = undefined;
        payload_slice.len = 1;
        payload_slice.ptr = @ptrCast([*]Payload, &this.payload);
        device.getQueue().writeBuffer(this.buffer, 0, payload_slice);
    }
};

pub const LightingResource = struct {
    pub const Payload = extern struct {
        origins: [1]zmath.Vec = .{zmath.f32x4(5.0, 5.0, 5.0, 0.0)},
        colors: [1]zmath.Vec = .{zmath.f32x4(1.0, 0.0, 0.0, 1.0)}
        //colors: [16][3]f32 = [_][3]f32{[_]f32{0.0} ** 3} ** 16,
        //padding: [140]u8 = [_]u8{0} ** 140
    };

    payload: Payload = .{},
    buffer: *gpu.Buffer = undefined,
    bg_layout: *gpu.BindGroupLayout = undefined,
    bg: *gpu.BindGroup = undefined,

    pub fn init(device: *gpu.Device) LightingResource {
        const queue = device.getQueue();
        var resource: LightingResource = .{};
            
        // Uniform buffer
        resource.buffer = device.createBuffer(&gpu.Buffer.Descriptor {
            .label = "LightingResource - uniform buffer",
            .mapped_at_creation = false,
            .size = @sizeOf(Payload),
            .usage = gpu.Buffer.UsageFlags {
                .copy_dst = true,
                .uniform = true
            }
        });

        // TODO: get this to load in from somewhere
        resource.payload = .{};
      
        var payload_slice: []Payload = undefined;
        payload_slice.len = 1;
        payload_slice.ptr = @ptrCast([*]Payload, &resource.payload);
        queue.writeBuffer(resource.buffer, 0, payload_slice);

        resource.bg_layout = device.createBindGroupLayout(&gpu.BindGroupLayout.Descriptor.init(.{
            .entries = &.{
                gpu.BindGroupLayout.Entry.buffer(
                    0,
                    gpu.ShaderStageFlags {
                        .vertex = true,
                        .fragment = true
                    },
                    gpu.Buffer.BindingType.uniform,
                    false,
                    @sizeOf(Payload)
                )
            }
        }));

        resource.bg = device.createBindGroup(&gpu.BindGroup.Descriptor.init(.{
            .layout = resource.bg_layout,
            .entries = &.{
                gpu.BindGroup.Entry.buffer(0, resource.buffer, 0, @sizeOf(Payload))
            }
        }));

        return resource;
    }
};

pub const MeshResource = struct {
    pub const BufferPayload = struct {
        buffer: []f32,
        texture: ?[]u8,
        texture_width: u16 = 0,
        texture_height: u16 = 0,
    };

    pub const UniformPayload = extern struct {
        model: zmath.Mat = zmath.identity(),
        normal: zmath.Mat = zmath.identity()
    };

    vertex_buffer: *gpu.Buffer,
    vertex_buffer_layout: gpu.VertexBufferLayout,

    uniform_buffer: *gpu.Buffer,

    texture: *gpu.Texture,

    bg_layout: *gpu.BindGroupLayout,
    bg: *gpu.BindGroup,

    pub fn init(gpa: std.mem.Allocator, device: *gpu.Device, path: []const u8, textured: bool) !MeshResource {
        const queue = device.getQueue();
        
        // FIXME: cmon man - sentinel b.s
        var file = std.fs.cwd().openFile(path, .{}) catch unreachable;
        defer file.close();
        const file_contents = file.readToEndAlloc(gpa, std.math.inf_u64) catch unreachable;
        defer gpa.free(file_contents);    
        const file_sentinel = gpa.alloc(u8, file_contents.len + 1) catch unreachable;
        defer gpa.free(file_sentinel);
        std.mem.copyForwards(u8, file_sentinel, file_contents);

        file_sentinel[file_contents.len] = 0;

        const model = model3d.load(file_sentinel[0..file_contents.len:0], null, null, null) orelse unreachable;
    
        const vertices: []model3d.Vertex = model.handle.vertex[0..model.handle.numvertex];

        const faces: []model3d.Face = model.handle.face[0..model.handle.numface];
        var buffer_toreturn = std.ArrayList(f32).init(gpa);
        for (faces) |face| {
            for (0..3) |i| {
                try buffer_toreturn.append(vertices[face.vertex[i]].x);
                try buffer_toreturn.append(vertices[face.vertex[i]].y);
                try buffer_toreturn.append(vertices[face.vertex[i]].z);
                try buffer_toreturn.append(vertices[face.normal[i]].x);
                try buffer_toreturn.append(vertices[face.normal[i]].y);
                try buffer_toreturn.append(vertices[face.normal[i]].z);
                if (textured) {    
                    try buffer_toreturn.append(model.handle.tmap[face.texcoord[i]].u);
                    try buffer_toreturn.append(1.0 - model.handle.tmap[face.texcoord[i]].v);
                }
            }
        }
        
        
        var texture_toreturn = std.ArrayList(u8).init(gpa);
        if (textured) {    
            var texture_data = model.textures()[0];
            std.debug.print("Texture size: {?}x{?}x{?}\n", .{texture_data.w, texture_data.h, texture_data.f});
            for (0..(@intCast(u32, texture_data.w) * @intCast(u32, texture_data.h))) |i| {
                try texture_toreturn.append(texture_data.d[i * 3]);
                try texture_toreturn.append(texture_data.d[i * 3 + 1]);
                try texture_toreturn.append(texture_data.d[i * 3 + 2]);
                try texture_toreturn.append(255);            
            }
        }

        var buffer_payload: BufferPayload = .{
            .buffer = buffer_toreturn.toOwnedSlice() catch unreachable,
            .texture = if (textured) (texture_toreturn.toOwnedSlice() catch unreachable) else null,
            .texture_width = if (textured) (model.textures()[0].w) else 0,
            .texture_height = if (textured) (model.textures()[0].h) else 0
        };

        var uniform_payload: UniformPayload = .{
            .model = zmath.rotationY(std.math.pi),
            .normal = zmath.inverse(zmath.transpose(zmath.rotationY(std.math.pi)))
        };

        var uniform_buffer = device.createBuffer(&.{
            .label = "Uniform buffer",
            .usage = gpu.Buffer.UsageFlags {
                .uniform = true,
                .copy_dst = true
            },
            .size = @sizeOf(UniformPayload)
        });
        
        var uniforms_slice: []UniformPayload = undefined;
        uniforms_slice.len = 1;
        uniforms_slice.ptr = @ptrCast([*]UniformPayload, &uniform_payload);
        queue.writeBuffer(uniform_buffer, 0, uniforms_slice);

        var vertex_buffer = device.createBuffer(&.{
            .label = "Vertex buffer",
            .usage = gpu.Buffer.UsageFlags {
                .vertex = true,
                .copy_dst = true
            },
            .size = buffer_payload.buffer.len * @sizeOf(f32)
        });
        queue.writeBuffer(vertex_buffer, 0, buffer_payload.buffer);
        
        var vertex_buffer_layout = gpu.VertexBufferLayout.init(.{
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
        });

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
                .height = buffer_payload.texture_height,
                .width = buffer_payload.texture_width
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
                .bytes_per_row = 4 * buffer_payload.texture_width,
                .rows_per_image = buffer_payload.texture_height,
                .offset = 0,
            },
            &gpu.Extent3D {
                .depth_or_array_layers = 1,
                .width = buffer_payload.texture_width,
                .height = buffer_payload.texture_height,
            },
            buffer_payload.texture.?
        );

        var bg_layout = device.createBindGroupLayout(&gpu.BindGroupLayout.Descriptor.init(.{
            .entries = &.{
                gpu.BindGroupLayout.Entry.buffer(
                    0, 
                    gpu.ShaderStageFlags {
                        .vertex = true,
                        .fragment = true
                    },
                    gpu.Buffer.BindingType.uniform,
                    false,
                    @sizeOf(UniformPayload)),
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

        var bg = device.createBindGroup(&gpu.BindGroup.Descriptor.init(.{
            .layout = bg_layout,
            .entries = &.{
                gpu.BindGroup.Entry.buffer(0, uniform_buffer, 0, @sizeOf(UniformPayload)),
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

        return .{
            .vertex_buffer = vertex_buffer,
            .vertex_buffer_layout = vertex_buffer_layout,
            .uniform_buffer = uniform_buffer,
            .texture = texture,
            .bg_layout = bg_layout,
            .bg = bg
        };
    }
};