const std = @import("std");
const gpu = @import("gpu");
const zmath = @import("zmath");
const model3d = @import("model3d");
const glfw = @import("glfw");

const Mat = zmath.Mat;
const Vec = zmath.Vec;

pub const ShaderResource = struct {
    module: *gpu.ShaderModule,

    pub fn init(gpa: std.mem.Allocator, device: *gpu.Device, path: []const u8) ShaderResource {
        var file = std.fs.cwd().openFile(path, .{ .mode = std.fs.File.OpenMode.read_only }) catch unreachable;
        defer file.close();
        const file_contents = file.readToEndAlloc(gpa, 1000000) catch unreachable;
        defer gpa.free(file_contents);    
        const shader_source = gpa.alloc(u8, file_contents.len + 1) catch unreachable;
        defer gpa.free(shader_source);
        std.mem.copyForwards(u8, shader_source, file_contents);
        shader_source[shader_source.len - 1] = 0;
        var shader_module = device.createShaderModuleWGSL("shaders", shader_source[0..(shader_source.len - 1) :0]);
        return .{
            .module = shader_module
        };
    }
};

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

    pub fn update_raw(this: *SceneResource, device: *gpu.Device, payload: Payload) void {
        this.payload = payload;

        var payload_slice: []Payload = undefined;
        payload_slice.len = 1;
        payload_slice.ptr = @ptrCast([*]Payload, &this.payload);
        device.getQueue().writeBuffer(this.buffer, 0, payload_slice);
    }
};

// One directional light for now.
pub const LightingResource = struct {
    pub const Payload = extern struct {
        direction: zmath.Vec = zmath.f32x4(5.0, 5.0, 5.0, 1.0),
        color: zmath.Vec = zmath.f32x4(1.0, 1.0, 1.0, 1.0),
        projection_matrix: zmath.Mat = zmath.identity()
    };

    payload: Payload = .{},
    buffer: *gpu.Buffer = undefined,
    bg_layout: *gpu.BindGroupLayout = undefined,
    bg: *gpu.BindGroup = undefined,

    pub fn init(device: *gpu.Device, direction: [3]f32, color: [3]f32) LightingResource {
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
        resource.payload = .{
            .direction = zmath.normalize3(zmath.f32x4(direction[0], direction[1], direction[2], 0.0)),
            .color = zmath.f32x4(color[0], color[1], color[2], 1.0),
        };
        
        resource.payload.projection_matrix = zmath.mul(
                zmath.orthographicOffCenterLh(-10.0, 10.0, 10.0, -10.0, -10.0, 10.0), 
                zmath.lookAtLh(zmath.inverse(resource.payload.direction), zmath.f32x4(0.0, 0.0, 0.0, 1.0), zmath.f32x4(0.0, 1.0, 0.0, 1.0)));

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

    buffer_payload: BufferPayload,
    uniform_payload: UniformPayload,

    vertex_buffer: *gpu.Buffer,
    vertex_buffer_count: u32,
    vertex_buffer_layout: gpu.VertexBufferLayout,

    uniform_buffer: *gpu.Buffer,

    texture: *gpu.Texture,

    bg_layout: *gpu.BindGroupLayout,
    bg: *gpu.BindGroup,

    pub fn init(gpa: std.mem.Allocator, device: *gpu.Device, path: []const u8, model_matrix: Mat) !MeshResource {
        const queue = device.getQueue();
        
        // FIXME: cmon man - sentinel b.s
        var file = std.fs.cwd().openFile(path, .{}) catch unreachable;
        defer file.close();
        const file_contents = file.readToEndAlloc(gpa, 1000000) catch unreachable;
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
            for (0..3) |i| { try buffer_toreturn.append(vertices[face.vertex[i]].x);
                try buffer_toreturn.append(vertices[face.vertex[i]].y);
                try buffer_toreturn.append(vertices[face.vertex[i]].z);
                try buffer_toreturn.append(vertices[face.normal[i]].x);
                try buffer_toreturn.append(vertices[face.normal[i]].y);
                try buffer_toreturn.append(vertices[face.normal[i]].z); 
                try buffer_toreturn.append(model.handle.tmap[face.texcoord[i]].u);
                try buffer_toreturn.append(1.0 - model.handle.tmap[face.texcoord[i]].v);
            }
        }
        
        
        var texture_toreturn = std.ArrayList(u8).init(gpa);    
        var texture_data = model.textures()[0];
        std.debug.print("Texture size: {?}x{?}x{?}\n", .{texture_data.w, texture_data.h, texture_data.f});
        for (0..(@intCast(u32, texture_data.w) * @intCast(u32, texture_data.h))) |i| {
            try texture_toreturn.append(texture_data.d[i * 3]);
            try texture_toreturn.append(texture_data.d[i * 3 + 1]);
            try texture_toreturn.append(texture_data.d[i * 3 + 2]);
            try texture_toreturn.append(255);            
        }
        

        var buffer_payload: BufferPayload = .{
            .buffer = buffer_toreturn.toOwnedSlice() catch unreachable,
            .texture = texture_toreturn.toOwnedSlice() catch unreachable,
            .texture_width = model.textures()[0].w,
            .texture_height = model.textures()[0].h
        };

        var uniform_payload: UniformPayload = .{
            .model = model_matrix,
            .normal = zmath.inverse(zmath.transpose(model_matrix))
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
            .buffer_payload = buffer_payload,
            .uniform_payload = uniform_payload,
            .vertex_buffer = vertex_buffer,
            .vertex_buffer_layout = vertex_buffer_layout,
            .vertex_buffer_count = @intCast(u32, buffer_payload.buffer.len) / 8,
            .uniform_buffer = uniform_buffer,
            .texture = texture,
            .bg_layout = bg_layout,
            .bg = bg
        };
    }
};


pub const UntexturedMeshResource = struct {
    pub const BufferPayload = struct {
        buffer: []f32
    };

    pub const UniformPayload = extern struct {
        model: zmath.Mat = zmath.identity(),
        normal: zmath.Mat = zmath.identity()
    };

    buffer_payload: BufferPayload,
    uniform_payload: UniformPayload,

    vertex_buffer: *gpu.Buffer,
    vertex_buffer_count: u32,
    vertex_buffer_layout: gpu.VertexBufferLayout,

    uniform_buffer: *gpu.Buffer,

    bg_layout: *gpu.BindGroupLayout,
    bg: *gpu.BindGroup,

    pub fn init(gpa: std.mem.Allocator, device: *gpu.Device, path: []const u8, model_matrix: Mat) !UntexturedMeshResource {
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
            }
        }
            
        var buffer_payload: BufferPayload = .{
            .buffer = buffer_toreturn.toOwnedSlice() catch unreachable
        };

        var uniform_payload: UniformPayload = .{
            .model = model_matrix,
            .normal = zmath.inverse(zmath.transpose(model_matrix))
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
        });

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
                    @sizeOf(UniformPayload))
            }
        }));

        var bg = device.createBindGroup(&gpu.BindGroup.Descriptor.init(.{
            .layout = bg_layout,
            .entries = &.{
                gpu.BindGroup.Entry.buffer(0, uniform_buffer, 0, @sizeOf(UniformPayload))
            }
        }));

        return .{
            .buffer_payload = buffer_payload,
            .uniform_payload = uniform_payload,
            .vertex_buffer = vertex_buffer,
            .vertex_buffer_layout = vertex_buffer_layout,
            .vertex_buffer_count = @intCast(u32, buffer_payload.buffer.len) / 6,
            .uniform_buffer = uniform_buffer,
            .bg_layout = bg_layout,
            .bg = bg
        };
    }
};
