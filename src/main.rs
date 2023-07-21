use std::time::Instant;

use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

mod camera;
mod common;
mod gbuffers;
mod loader;
mod passes;
mod renderer;
mod resources;
mod spring;
mod tangent_generation;
mod texture;

use renderer::Renderer;

// I HATE ASYNC! I HATE ASYNC!
pub async fn run() {
    env_logger::init();
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_inner_size(winit::dpi::PhysicalSize::new(1600, 900))
        .with_resizable(false)
        .build(&event_loop)
        .unwrap();

    let mut app = Renderer::new(&window).await;
    let mut last_render_time = Instant::now();
    event_loop.run(move |event, _, control_flow| match event {
        Event::WindowEvent {
            ref event,
            window_id,
        } if window_id == window.id() => {
            app.input(event);
            match event {
                WindowEvent::CloseRequested
                | WindowEvent::KeyboardInput {
                    input:
                        KeyboardInput {
                            state: ElementState::Pressed,
                            virtual_keycode: Some(VirtualKeyCode::Escape),
                            ..
                        },
                    ..
                } => *control_flow = ControlFlow::Exit,
                WindowEvent::MouseInput {
                    button: MouseButton::Right,
                    state,
                    ..
                } => {
                    if *state == ElementState::Released {
                        let _ = window.set_cursor_grab(winit::window::CursorGrabMode::None);
                    } else if *state == ElementState::Pressed {
                        let _ = window.set_cursor_grab(winit::window::CursorGrabMode::Locked);
                    }
                }
                _ => {}
            }
        }
        Event::RedrawRequested(window_id) if window_id == window.id() => {
            let now = Instant::now();
            let dt = now - last_render_time;
            last_render_time = now;
            app.update(dt);
            match app.render() {
                Ok(_) => {}
                Err(e) => eprintln!("{:?}", e),
            }
        }
        Event::MainEventsCleared => {
            window.request_redraw();
        }
        _ => {}
    });
}

fn main() {
    pollster::block_on(run());
}
