use legion::World;
use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

use crate::{
    renderer::Renderer, 
    text::TextBoxFactory, ui_box::UiBoxFactory,
};

fn initialize_world(renderer: &mut Renderer, world: &mut World) {
    let text_factory = TextBoxFactory::new(renderer).unwrap();
    let text_components = text_factory.create("fn main() -> String {}", (0f32, 0f32), 0.1);
    world.extend(text_components);

    let ui_box_factory = UiBoxFactory::new(renderer).unwrap();
    let ui_box = ui_box_factory.create("#122630",(0f32, 0f32), (0.5, 0.5), 0.9).unwrap();
    world.push(ui_box);
}

pub fn run() {
    env_logger::init();
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop).unwrap();

    let mut renderer = Renderer::new(&window);
    let mut world = World::default();

    initialize_world(&mut renderer, &mut world);

    event_loop.run(move |event, _, control_flow| match event {
        Event::WindowEvent {
            ref event,
            window_id,
        } if window_id == window.id() => match event {
            WindowEvent::Resized(new_size) => renderer.resize(*new_size),
            WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                renderer.resize(**new_inner_size)
            }
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
            _ => {}
        },
        Event::RedrawRequested(_) => {
            match renderer.render(&world) {
                Ok(_) => {}
                // Reconfigure the surface if lost
                Err(wgpu::SurfaceError::Lost) => renderer.find_display(),
                // The system is out of memory, we should probably quit
                Err(wgpu::SurfaceError::OutOfMemory) => *control_flow = ControlFlow::Exit,
                // All other errors (Outdated, Timeout) should be resolved by the next frame
                Err(e) => eprintln!("{:?}", e),
            }
        }
        Event::MainEventsCleared => {
            window.request_redraw();
        }
        _ => {}
    });
}
