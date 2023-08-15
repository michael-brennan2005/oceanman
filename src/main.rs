use std::time::Instant;

use clap::Parser;
use egui_wgpu::renderer::ScreenDescriptor;
use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

mod camera;
mod common;
mod cubemap;
mod gbuffers;
mod loader;
mod passes;
mod renderer;
mod resources;
mod shadowmap;
mod spring;
mod tangent_generation;
mod texture;

use renderer::Renderer;

#[derive(Parser)]
pub struct RendererConfig {
    /// gltf scene to load
    #[arg(short, long)]
    pub gltf: Option<String>,
    /// skybox to load
    #[arg(short, long)]
    pub skybox: Option<String>,
    /// irradiance (diffuse) to load
    #[arg(short, long)]
    pub irradiance: Option<String>,
    /// prefilter (specular) to load
    #[arg(short, long)]
    pub prefilter: Option<String>,
}

// I HATE ASYNC! I HATE ASYNC!
pub async fn run() {
    let args = RendererConfig::parse();

    env_logger::init();
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_inner_size(winit::dpi::PhysicalSize::new(1600, 900))
        .with_resizable(false)
        .build(&event_loop)
        .unwrap();

    let mut egui_state = egui_winit::State::new(&event_loop);
    let egui_context = egui::Context::default();
    let egui_screen_descriptor = ScreenDescriptor {
        size_in_pixels: [1600, 900],
        pixels_per_point: window.scale_factor() as f32,
    };
    let mut app = Renderer::new(&window, &args);
    let mut last_render_time = Instant::now();
    event_loop.run(move |event, _, control_flow| match event {
        Event::WindowEvent {
            ref event,
            window_id,
        } if window_id == window.id() => {
            egui_state.on_event(&egui_context, event);
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
            let raw_input = egui_state.take_egui_input(&window);
            let ui_output = egui_context.run(raw_input, |ctx| {
                app.ui(ctx);
            });

            egui_state.handle_platform_output(&window, &egui_context, ui_output.platform_output);
            let clipped_primitives = egui_context.tessellate(ui_output.shapes);

            let now = Instant::now();
            let dt = now - last_render_time;
            last_render_time = now;
            app.update(dt);
            match app.render(
                &ui_output.textures_delta,
                &clipped_primitives,
                &egui_screen_descriptor,
            ) {
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
