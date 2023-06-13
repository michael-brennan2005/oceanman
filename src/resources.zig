const std = @import("std");
const gpu = @import("gpu");
const zmath = @import("zmath");
const glfw = @import("glfw");

const Mat = zmath.Mat;
const Vec = zmath.Vec;

pub const SceneResource = struct {
    pub const Payload = extern struct {
        perspective: Mat = zmath.identity(),
        view: Mat = zmath.identity(),
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