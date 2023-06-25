use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

mod camera;
mod loader;
mod renderer;
mod resources;

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
                        let result = window.set_cursor_grab(winit::window::CursorGrabMode::None);
                        if result.is_err() {
                            println!("error: {:?}", result);
                        }
                    } else if *state == ElementState::Pressed {
                        let result = window.set_cursor_grab(winit::window::CursorGrabMode::Locked);
                        if result.is_err() {
                            println!("error: {:?}", result);
                        }
                    }
                }
                _ => {}
            }
        }
        Event::RedrawRequested(window_id) if window_id == window.id() => {
            app.update();
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
