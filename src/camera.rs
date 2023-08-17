use std::time::Duration;

use glam::{vec3, vec4, Mat4, Vec3, Vec4};
use winit::event::{ElementState, KeyboardInput, MouseButton, VirtualKeyCode, WindowEvent};

use crate::spring::Spring;

pub struct Camera {
    pub eye: Vec3,
    front: Vec3,
    up: Vec3,
    aspect: f32,
    fovy: f32,
    znear: f32,
    zfar: f32,
    pub pitch: f32,
    pub yaw: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            eye: vec3(-3.0, 3.0, -3.0),
            front: (vec3(0.0, 0.0, 0.0) - vec3(-3.0, 3.0, -3.0)).normalize(),
            up: vec3(0.0, 1.0, 0.0),
            aspect: 1600.0 / 900.0,
            fovy: 45.0_f32.to_radians(),
            znear: 0.01,
            zfar: 100.0,
            pitch: 0.0,
            yaw: 90.0,
        }
    }
}

impl Camera {
    pub fn build_uniforms(&self) -> (Mat4, Vec4) {
        let view = Mat4::look_to_lh(self.eye, self.front, self.up);
        (view, vec4(self.eye.x, self.eye.y, self.eye.z, 1.0))
    }
}

pub trait CameraController {
    fn input(&mut self, event: &WindowEvent);
    fn update(&mut self, camera: &mut Camera, dt: Duration);
    fn ui(&mut self, camera: &mut Camera, ui: &mut egui::Ui) {}
}

pub struct FlyingCamera {
    direction: Vec3,
    x_spring: Spring<f32>,
    y_spring: Spring<f32>,
    z_spring: Spring<f32>,
    max_speed: f32,

    right_click: bool,
    first_mouse: bool,
    last_x: f32,
    last_y: f32,
    pub pitch: f32,
    pub yaw: f32,
}

impl FlyingCamera {
    pub fn new() -> Self {
        FlyingCamera {
            direction: vec3(0.0, 0.0, 1.0),
            x_spring: Spring::new(5.0, 5.0, 0.0),
            y_spring: Spring::new(5.0, 5.0, 0.0),
            z_spring: Spring::new(5.0, 5.0, 0.0),

            max_speed: 10.0,

            right_click: false,
            first_mouse: false,
            last_x: 0.0,
            last_y: 0.0,
            pitch: 0.0,
            yaw: 90.0,
        }
    }
}
impl CameraController for FlyingCamera {
    fn input(&mut self, event: &WindowEvent) {
        match event {
            WindowEvent::KeyboardInput {
                input:
                    KeyboardInput {
                        state,
                        virtual_keycode: Some(keycode),
                        ..
                    },
                ..
            } => {
                let is_pressed = *state == ElementState::Pressed;
                match keycode {
                    VirtualKeyCode::W | VirtualKeyCode::Up => {
                        self.z_spring.goal = if is_pressed { 1.0 } else { 0.0 };
                    }
                    VirtualKeyCode::A | VirtualKeyCode::Left => {
                        self.x_spring.goal = if is_pressed { -1.0 } else { 0.0 };
                    }
                    VirtualKeyCode::S | VirtualKeyCode::Down => {
                        self.z_spring.goal = if is_pressed { -1.0 } else { 0.0 };
                    }
                    VirtualKeyCode::D | VirtualKeyCode::Right => {
                        self.x_spring.goal = if is_pressed { 1.0 } else { 0.0 };
                    }
                    VirtualKeyCode::E => {
                        self.y_spring.goal = if is_pressed { 1.0 } else { 0.0 };
                    }
                    VirtualKeyCode::Q => {
                        self.y_spring.goal = if is_pressed { -1.0 } else { 0.0 };
                    }
                    _ => {}
                }
            }
            WindowEvent::MouseInput {
                device_id: _,
                state,
                button: MouseButton::Right,
                modifiers: _,
            } => {
                if *state == ElementState::Pressed {
                    self.right_click = true;
                } else {
                    self.right_click = false;
                }
            }
            WindowEvent::CursorMoved {
                device_id: _,
                position,
                modifiers: _,
            } => {
                if !self.right_click {
                    self.first_mouse = true;
                    return;
                }

                if self.first_mouse {
                    self.last_x = position.x as f32;
                    self.last_y = position.y as f32;
                    self.first_mouse = false;
                }

                let mut x_offset: f32 = position.x as f32 - self.last_x;
                let mut y_offset: f32 = position.y as f32 - self.last_y;
                self.last_x = position.x as f32;
                self.last_y = position.y as f32;

                let sensitivity = 0.2_f32;
                x_offset *= sensitivity;
                y_offset *= sensitivity;
                self.yaw -= x_offset;
                self.pitch -= y_offset;

                self.pitch = self.pitch.clamp(-89.0, 89.0);
                self.direction = vec3(
                    f32::cos(self.yaw.to_radians()) * f32::cos(self.pitch.to_radians()),
                    f32::sin(self.pitch.to_radians()),
                    f32::sin(self.yaw.to_radians()) * f32::cos(self.pitch.to_radians()),
                );
            }
            _ => {}
        }
    }

    fn update(&mut self, camera: &mut Camera, dt: Duration) {
        let dt_seconds = dt.as_secs_f32();
        camera.front = self.direction;

        camera.eye += camera.front * self.z_spring.update(dt_seconds) * self.max_speed * dt_seconds;
        camera.eye += Vec3::normalize(Vec3::cross(camera.up, camera.front))
            * self.x_spring.update(dt_seconds)
            * self.max_speed
            * dt_seconds;

        let right = Vec3::normalize(Vec3::cross(camera.up, camera.front));
        camera.eye += Vec3::normalize(Vec3::cross(camera.front, right))
            * self.y_spring.update(dt_seconds)
            * self.max_speed
            * dt_seconds;
    }

    fn ui(&mut self, camera: &mut Camera, ui: &mut egui::Ui) {
        egui::CollapsingHeader::new("Camera").show(ui, |ui| {
            ui.label(format!(
                "Position: {:.3} {:.3} {:.3}\nYaw: {:.3}\nPitch: {:.3}",
                camera.eye.x, camera.eye.y, camera.eye.z, self.yaw, self.pitch
            ));

            ui.add(
                egui::Slider::new(&mut self.max_speed, 0.0..=10.0)
                    .text("Camera speed")
                    .show_value(true),
            );
        });
    }
}
