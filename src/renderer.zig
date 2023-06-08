const std = @import("std");
const gpu = @import("gpu");
const zmath = @import("zmath");
const glfw = @import("glfw");

const Mat = zmath.Mat;
const Vec = zmath.Vec;

const Model = @import("loader.zig").Model;

const Uniforms = struct {
    perspective: Mat,
    view: Mat,
    model: Mat
};

const Camera = struct {
    right_click: bool = false,
    first_mouse: bool = true,
    yaw: f32 = 0.0,
    pitch: f32 = 0.0,
    last_x: f32 = 0.0,
    last_y: f32 = 0.0,
    
    forward: bool = false,
    backward: bool = false,
    left: bool = false,
    right: bool = false,
    upp: bool = false,
    down: bool = false,

    position: Vec = zmath.f32x4(0.0, 0.0, 3.0, 1.0),
    front: Vec = zmath.f32x4(0.0, 0.0, -1.0, 0.0),
    up: Vec = zmath.f32x4(0.0, 1.0, 0.0, 1.0),

    pub fn update(this: *Camera, dt: f32) void {
        const cameraSpeed: f32 = 2.5;
        if (this.forward) {
            // this shouldn't be a plus (should be -), could be weird LH vs RH issue
            this.position += this.front * @splat(4, cameraSpeed * dt);
        }
        if (this.backward) {
            this.position -= this.front * @splat(4, cameraSpeed * dt);
        }
        if (this.left) {
            this.position -= zmath.normalize3(zmath.cross3(this.up, this.front)) * @splat(4, cameraSpeed * dt);
        }
        if (this.right) {
            this.position += zmath.normalize3(zmath.cross3(this.up, this.front)) * @splat(4, cameraSpeed * dt);
        }
        if (this.upp) {
            this.position -= this.up * @splat(4, cameraSpeed * dt); 
        }
        if (this.down) {
            this.position += this.up * @splat(4, cameraSpeed * dt);
        }
    }
};

const Renderer = @This();
const log = std.log.scoped(.oceanman);

device: *gpu.Device,
surface: *gpu.Surface,
swapchain: *gpu.SwapChain,
queue: *gpu.Queue,
pipeline: *gpu.RenderPipeline,

uniforms: Uniforms,
uniform_buffer: *gpu.Buffer,
uniform_binding: *gpu.BindGroup,

depth_texture: *gpu.Texture,

vertex_buffer: *gpu.Buffer,
vertex_count: usize,
index_buffer: *gpu.Buffer,
index_count: usize,

camera: Camera,

// MARK: input
pub fn onKeyDown(this: *Renderer, key: glfw.Key) void {
    switch (key) {
        glfw.Key.w => {
            this.camera.forward = true;                    
        },
        glfw.Key.a => {
            this.camera.left = true;
        },
        glfw.Key.s => {
            this.camera.backward = true;
        },
        glfw.Key.d => {
            this.camera.right = true;
        },
        glfw.Key.q => {
            this.camera.upp = true;
        },
        glfw.Key.e => {
            this.camera.down = true;
        },
        else => return
    }
}

pub fn onKeyUp(this: *Renderer, key: glfw.Key) void {
    switch (key) {
        glfw.Key.w => {
            this.camera.forward = false;                    
        },
        glfw.Key.a => {
            this.camera.left = false;
        },
        glfw.Key.s => {
            this.camera.backward = false;
        },
        glfw.Key.d => {
            this.camera.right = false;
        },
        glfw.Key.q => {
            this.camera.upp = false;
        },
        glfw.Key.e => {
            this.camera.down = false;
        },
        else => return
    }
}

pub fn onMouseButtonDown(this: *Renderer, window: *const glfw.Window, key: glfw.MouseButton) void {
    if (key == glfw.MouseButton.right) {
        window.setInputModeCursor(glfw.Window.InputModeCursor.disabled);
        this.camera.right_click = true;
    }
}

pub fn onMouseButtonUp(this: *Renderer, window: *const glfw.Window, key: glfw.MouseButton) void {
    if (key == glfw.MouseButton.right) {
        window.setInputModeCursor(glfw.Window.InputModeCursor.normal);
        this.camera.right_click = false;
    }
}

pub fn onMouseMove(this: *Renderer, x: f32, y: f32) void {
    if (!this.camera.right_click) {
        this.camera.first_mouse = true;
        return;
    }

    if (this.camera.first_mouse) {
        this.camera.last_x = @floatCast(f32, x);
        this.camera.last_y = @floatCast(f32, y);
        this.camera.first_mouse = false;
    }

    var x_offset: f32 = x - this.camera.last_x;
    var y_offset: f32 = y - this.camera.last_y;
    this.camera.last_x = x;
    this.camera.last_y = y;

    var sensitivity: f32 = 0.1;
    x_offset *= sensitivity;
    y_offset *= sensitivity;

    this.camera.yaw -= x_offset;
    this.camera.pitch -= y_offset;

    if (this.camera.pitch > 89.0) {
        this.camera.pitch = 89.0;
    } 

    if (this.camera.pitch < -89.0) {
        this.camera.pitch = -89.0;
    }

    const radians = std.math.degreesToRadians;
    var dir = zmath.f32x4(
        @cos(radians(f32, this.camera.yaw)) * @cos(radians(f32, this.camera.pitch)),
        @sin(radians(f32, this.camera.pitch)),
        @sin(radians(f32, this.camera.yaw)) * @cos(radians(f32, this.camera.pitch)),
        0.0
    );
    this.camera.front = zmath.normalize3(dir);
}

// MARK: init
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
    const ratio: f32  = 640.0 / 480.0;

    var model = Model.createFromFile(gpa, "resources/suzanne.m3d") catch unreachable;
    
    // Write vertex and index buffers
    var vertex_buffer = device.createBuffer(&.{
        .usage = gpu.Buffer.UsageFlags {
            .vertex = true,
            .copy_dst = true
        },
        .size = model.vertices.len * @sizeOf(f32)
    });
    queue.writeBuffer(vertex_buffer, 0, model.vertices);

    var index_buffer = device.createBuffer(&.{
        .usage = gpu.Buffer.UsageFlags {
            .index = true,
            .copy_dst = true
        },
        .size = model.indices.len * @sizeOf(u32)
    });
    queue.writeBuffer(index_buffer, 0, model.indices);

    // Write uniform buffers and binding group.
    var uniforms = Uniforms {
        .model = zmath.identity(),
        .view = zmath.identity(),
        .perspective = zmath.perspectiveFovLh(1.22, ratio, 0.01, 100.0)
    };
    var uniform_buffer = device.createBuffer(&.{
        .usage = gpu.Buffer.UsageFlags {
            .uniform = true,
            .copy_dst = true
        },
        .size = @sizeOf(Uniforms)
    });

    var uniforms_slice: []Uniforms = undefined;
    uniforms_slice.len = 1;
    uniforms_slice.ptr = @ptrCast([*]Uniforms, &uniforms);
    queue.writeBuffer(uniform_buffer, 0, uniforms_slice);

    var uniform_layout = device.createBindGroupLayout(&gpu.BindGroupLayout.Descriptor.init(.{
        .entries = &.{
            gpu.BindGroupLayout.Entry.buffer(
                    0, 
                    gpu.ShaderStageFlags {
                        .vertex = true
                    },
                    gpu.Buffer.BindingType.uniform,
                    false,
                    @sizeOf(Uniforms))
        }
    }));
    var uniform_binding = device.createBindGroup(&gpu.BindGroup.Descriptor.init(.{
        .layout = uniform_layout,
        .entries = &.{
            gpu.BindGroup.Entry.buffer(0, uniform_buffer, 0, @sizeOf(Uniforms)),
        }
    }));

    // Depth texture
    const depth_texture = device.createTexture(&gpu.Texture.Descriptor.init(.{
        .usage = gpu.Texture.UsageFlags {
            .render_attachment = true
        },
        .dimension = gpu.Texture.Dimension.dimension_2d,
        .format = gpu.Texture.Format.depth24_plus,
        .size = gpu.Extent3D {
            .depth_or_array_layers = 1,
            .width = 640,
            .height = 480
        },
        .view_formats = &.{
            gpu.Texture.Format.depth24_plus
        },
    }));

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
        .layout = device.createPipelineLayout(&gpu.PipelineLayout.Descriptor.init(.{
            .bind_group_layouts = &.{ uniform_layout }
        })),
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
        .depth_stencil = &.{
            .format = gpu.Texture.Format.depth24_plus,
            .depth_compare = gpu.CompareFunction.less,
            .depth_write_enabled = true,
            .stencil_read_mask = 0,
            .stencil_write_mask = 0
        },
        .multisample = .{}
    });

    log.info("renderer initialized!", .{});
    return .{ 
        .device = device, 
        .surface = surface,
        .swapchain = swapchain,
        .queue = queue,
        .pipeline = pipeline,
        .uniforms = uniforms,
        .uniform_buffer = uniform_buffer,
        .uniform_binding = uniform_binding,
        .depth_texture = depth_texture,
        .vertex_buffer = vertex_buffer,
        .vertex_count = model.vertices.len,
        .index_buffer = index_buffer,
        .index_count = model.indices.len,
        .camera = .{}
    };

}

// MARK: update
pub fn update(this: *Renderer, dt: f32) void {
    this.camera.update(dt);
    this.uniforms.view = zmath.lookAtLh(this.camera.position, this.camera.position + this.camera.front, this.camera.up);

    var uniforms_slice: []Uniforms = undefined;
    uniforms_slice.len = 1;
    uniforms_slice.ptr = @ptrCast([*]Uniforms, &this.uniforms);
    this.queue.writeBuffer(this.uniform_buffer, 0, uniforms_slice);
    
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
        .depth_stencil_attachment = &gpu.RenderPassDepthStencilAttachment {
            .view = this.depth_texture.createView(&gpu.TextureView.Descriptor {
                .aspect = gpu.Texture.Aspect.depth_only,
                .base_array_layer = 0,
                .array_layer_count = 1,
                .base_mip_level = 0,
                .mip_level_count = 1,
                .dimension = gpu.TextureView.Dimension.dimension_2d,
                .format = gpu.Texture.Format.depth24_plus
            }),
            .depth_clear_value = 1.0,
            .depth_load_op = gpu.LoadOp.clear,
            .depth_store_op = gpu.StoreOp.store,
            .depth_read_only = false
        }
    }));
    defer renderPass.release();

    renderPass.setPipeline(this.pipeline);
    renderPass.setBindGroup(0, this.uniform_binding, null);
    renderPass.setVertexBuffer(0, this.vertex_buffer, 0, this.vertex_count * @sizeOf(f32));
    renderPass.setIndexBuffer(this.index_buffer, gpu.IndexFormat.uint32, 0, this.index_count * @sizeOf(u32));

    renderPass.drawIndexed(@intCast(u32, this.index_count), 1, 0, 0, 0);
    renderPass.end();
        
    var commands = encoder.finish(&.{});
    defer commands.release();

    this.queue.submit(&.{commands});
    this.swapchain.present();
}

// MARK: deinit
pub fn deinit(this: *Renderer) void {
    _ = this;
    log.info("deinitializing renderer", .{});
}

