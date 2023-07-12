use glam::{vec3, vec4, Mat4, Vec3, Vec4};
use winit::event::{ElementState, KeyboardInput, MouseButton, VirtualKeyCode, WindowEvent};

// TODO: holy shit there has to be a better way this is too many variables
// TODO: this code is pretty much a straight - port of what was from zig + shoehorning in to fit how winit works. better, more rust-y way?
pub struct Camera {
    eye: Vec3,
    front: Vec3,
    up: Vec3,
    aspect: f32,
    fovy: f32,
    znear: f32,
    zfar: f32,
    speed: f32,
    is_forward_pressed: bool,
    is_backward_pressed: bool,
    is_left_pressed: bool,
    is_right_pressed: bool,
    is_up_pressed: bool,
    is_down_pressed: bool,
    right_click: bool,
    first_mouse: bool,
    last_x: f32,
    last_y: f32,
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
            speed: 0.04,
            is_forward_pressed: false,
            is_backward_pressed: false,
            is_left_pressed: false,
            is_right_pressed: false,
            is_up_pressed: false,
            is_down_pressed: false,
            right_click: false,
            last_x: 0.0,
            last_y: 0.0,
            first_mouse: true,
            pitch: 0.0,
            yaw: 90.0,
        }
    }
}

impl Camera {
    pub fn build_uniforms(&self) -> (Mat4, Vec4) {
        let perspective_view = Mat4::perspective_lh(self.fovy, self.aspect, self.znear, self.zfar)
            * Mat4::look_to_lh(self.eye, self.front, self.up);
        (
            perspective_view,
            vec4(self.eye.x, self.eye.y, self.eye.z, 1.0),
        )
    }

    pub fn input(&mut self, event: &WindowEvent) {
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
                        self.is_forward_pressed = is_pressed;
                    }
                    VirtualKeyCode::A | VirtualKeyCode::Left => {
                        self.is_left_pressed = is_pressed;
                    }
                    VirtualKeyCode::S | VirtualKeyCode::Down => {
                        self.is_backward_pressed = is_pressed;
                    }
                    VirtualKeyCode::D | VirtualKeyCode::Right => {
                        self.is_right_pressed = is_pressed;
                    }
                    VirtualKeyCode::E => {
                        self.is_up_pressed = is_pressed;
                    }
                    VirtualKeyCode::Q => {
                        self.is_down_pressed = is_pressed;
                    }
                    _ => {}
                }
            }
            WindowEvent::MouseInput {
                device_id,
                state,
                button: MouseButton::Right,
                modifiers,
            } => {
                if *state == ElementState::Pressed {
                    self.right_click = true;
                } else {
                    self.right_click = false;
                }
            }
            WindowEvent::CursorMoved {
                device_id,
                position,
                modifiers,
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
                let direction = vec3(
                    f32::cos(self.yaw.to_radians()) * f32::cos(self.pitch.to_radians()),
                    f32::sin(self.pitch.to_radians()),
                    f32::sin(self.yaw.to_radians()) * f32::cos(self.pitch.to_radians()),
                );

                self.front = Vec3::normalize(direction);
            }
            _ => {}
        }
    }

    pub fn update(&mut self) {
        if self.is_forward_pressed {
            self.eye += self.front * self.speed;
        }

        if self.is_backward_pressed {
            self.eye -= self.front * self.speed;
        }

        if self.is_left_pressed {
            self.eye -= Vec3::normalize(Vec3::cross(self.up, self.front)) * self.speed;
        }

        if self.is_right_pressed {
            self.eye += Vec3::normalize(Vec3::cross(self.up, self.front)) * self.speed;
        }

        if self.is_up_pressed {
            let right = Vec3::normalize(Vec3::cross(self.up, self.front));
            self.eye -= Vec3::normalize(Vec3::cross(right, self.front)) * self.speed;
        }

        if self.is_down_pressed {
            let right = Vec3::normalize(Vec3::cross(self.up, self.front));
            self.eye += Vec3::normalize(Vec3::cross(right, self.front)) * self.speed;
        }
    }
}
