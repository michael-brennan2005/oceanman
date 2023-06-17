const std = @import("std");
const gpu = @import("gpu");
const zmath = @import("zmath");
const glfw = @import("glfw");

const Mat = zmath.Mat;
const Vec = zmath.Vec;

const Camera = @import("camera.zig").Camera;

const loadFromFile = @import("loader.zig").loadFromFile;

const MeshPipeline = @import("pipelines.zig").MeshPipeline;

const SceneResource = @import("resources.zig").SceneResource;
const LightingResource = @import("resources.zig").LightingResource;
const MeshResource = @import("resources.zig").MeshResource;

const Renderer = @This();
const log = std.log.scoped(.oceanman);

device: *gpu.Device,
surface: *gpu.Surface,
swapchain: *gpu.SwapChain,
queue: *gpu.Queue,

depth_texture: *gpu.Texture,
depth_texture_view: *gpu.TextureView,

shadowmap_texture: *gpu.Texture,
shadowmap_texture_view: *gpu.TextureView,

dummy_texture: *gpu.Texture,

camera: Camera,

mesh_pipeline: *MeshPipeline,

scene_resource: *SceneResource,
lighting_resource: *LightingResource,
mesh_resources: []*MeshResource,

// MARK: input/glfw callbacks
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
pub fn init(gpa: std.mem.Allocator, device: *gpu.Device, surface: *gpu.Surface, path: []const u8) Renderer {
    log.info("initializng renderer...", .{});
    
    const queue = device.getQueue();

    // Swapchain
    var swapchain = device.createSwapChain(surface, &gpu.SwapChain.Descriptor {
        .width = 1600,
        .height = 900,
        .usage = gpu.Texture.UsageFlags {
            .render_attachment = true
        },
        .present_mode = gpu.PresentMode.fifo,
        .format = gpu.Texture.Format.bgra8_unorm
    });

    // Depth texture
    const depth_texture = device.createTexture(&gpu.Texture.Descriptor.init(.{
        .label = "Depth texture",
        .usage = gpu.Texture.UsageFlags {
            .render_attachment = true
        },
        .dimension = gpu.Texture.Dimension.dimension_2d,
        .format = gpu.Texture.Format.depth24_plus,
        .size = gpu.Extent3D {
            .depth_or_array_layers = 1,
            .width = 1600,
            .height = 900
        },
        .view_formats = &.{
            gpu.Texture.Format.depth24_plus
        },
    }));

    const depth_texture_view = depth_texture.createView(&gpu.TextureView.Descriptor {
        .label = "Depth texture view",
        .aspect = gpu.Texture.Aspect.depth_only,
        .base_array_layer = 0,
        .array_layer_count = 1,
        .base_mip_level = 0,
        .mip_level_count = 1,
        .dimension = gpu.TextureView.Dimension.dimension_2d,
        .format = gpu.Texture.Format.depth24_plus
    });

    // Shadowmap texture
    const shadowmap_texture = device.createTexture(&gpu.Texture.Descriptor.init(.{
        .label = "Shadow texture",
        .usage = gpu.Texture.UsageFlags {
            .render_attachment = true,
            .texture_binding = true
        },
        .dimension = gpu.Texture.Dimension.dimension_2d,
        .format = gpu.Texture.Format.depth24_plus,
        .size = gpu.Extent3D {
            .depth_or_array_layers = 1,
            .width = 1600,
            .height = 900
        },
        .view_formats = &.{
            gpu.Texture.Format.depth24_plus
        }
    }));

    const shadowmap_texture_view = shadowmap_texture.createView(&gpu.TextureView.Descriptor {
        .label = "Shadow texture view",
        .aspect = gpu.Texture.Aspect.depth_only,
        .base_array_layer = 0,
        .array_layer_count = 1,
        .base_mip_level = 0,
        .mip_level_count = 1,
        .dimension = gpu.TextureView.Dimension.dimension_2d,
        .format = gpu.Texture.Format.depth24_plus
    });

    var resources = loadFromFile(gpa, device, path);
        
    var scene_resource = gpa.create(SceneResource) catch unreachable;
    scene_resource.* = SceneResource.init(device);

    var mesh_pipeline = gpa.create(MeshPipeline) catch unreachable;
    mesh_pipeline.* = MeshPipeline.init(gpa, device, queue, scene_resource, resources.lighting, resources.meshes);

    log.info("renderer initialized!", .{});
    return .{ 
        .device = device, 
        .surface = surface,
        .swapchain = swapchain,
        .queue = queue,
        .depth_texture = depth_texture,
        .depth_texture_view = depth_texture_view,
        .mesh_pipeline = mesh_pipeline,
        .lighting_resource = resources.lighting,
        .scene_resource = scene_resource,
        .mesh_resources = resources.meshes,
        .shadowmap_texture = shadowmap_texture,
        .shadowmap_texture_view = shadowmap_texture_view,
        .camera = .{}
    };

}

// MARK: update
pub fn update(this: *Renderer, dt: f32) void {
    this.device.tick();
    this.camera.update(dt);

    var next_texture = this.swapchain.getCurrentTextureView();
    defer next_texture.release();

    var encoder = this.device.createCommandEncoder(&.{});
    defer encoder.release();
    
    var shadowPass = encoder.beginRenderPass(&gpu.RenderPassDescriptor.init(.{
        .label = "Shadow Pass",
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
    defer shadowPass.release();
    
    this.mesh_pipeline.update(shadowPass);

    shadowPass.end();
    //var renderPass = encoder.beginRenderPass(&gpu.RenderPassDescriptor.init(.{
    //    .label = "Pass",
    //    .color_attachments = &.{
    //        gpu.RenderPassColorAttachment {
    //            .clear_value = gpu.Color { .r = 0.0, .g = 0.0, .b = 0.0, .a = 1.0 },
    //            .load_op = gpu.LoadOp.clear,
    //            .store_op = gpu.StoreOp.store,
    //            .view = next_texture
    //        }
    //    },
    //    .depth_stencil_attachment = &gpu.RenderPassDepthStencilAttachment {
    //        .view = this.depth_texture.createView(&gpu.TextureView.Descriptor {
    //            .aspect = gpu.Texture.Aspect.depth_only,
    //            .base_array_layer = 0,
    //            .array_layer_count = 1,
    //            .base_mip_level = 0,
    //            .mip_level_count = 1,
    //            .dimension = gpu.TextureView.Dimension.dimension_2d,
    //            .format = gpu.Texture.Format.depth24_plus
    //        }),
    //        .depth_clear_value = 1.0,
    //        .depth_load_op = gpu.LoadOp.clear,
    //        .depth_store_op = gpu.StoreOp.store,
    //        .depth_read_only = false
    //    }
    //}));
    //defer renderPass.release();
//
    //this.scene_resource.update(this.device, 1600.0 / 900.0, this.camera.position, this.camera.position + this.camera.front, this.camera.up);
//
    //this.mesh_pipeline.update(renderPass);
    //
    //renderPass.end();
        
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

