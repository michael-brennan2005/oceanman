const std = @import("std");
const gpu = @import("gpu");
const zmath = @import("zmath");
const glfw = @import("glfw");

const Mat = zmath.Mat;
const Vec = zmath.Vec;

pub const Camera = @This();

right_click: bool = false,
first_mouse: bool = true,
yaw: f32 = 90.0,
pitch: f32 = 0.0,
last_x: f32 = 0.0,
last_y: f32 = 0.0,

forward: bool = false,
backward: bool = false,
left: bool = false,
right: bool = false,
upp: bool = false,
down: bool = false,

position: Vec = zmath.f32x4(0.0, 0.0, -3.0, 1.0),
front: Vec = zmath.f32x4(0.0, 0.0, 1.0, 0.0),
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