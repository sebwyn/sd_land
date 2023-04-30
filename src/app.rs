use std::env::{Args, self};

use legion::World;
use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder, dpi::PhysicalSize,
};

use crate::{
    renderer::Renderer, 
    text::TextBoxFactory, camera::Camera
};

fn initialize_world(renderer: &mut Renderer, world: &mut World) {
    let text_factory = TextBoxFactory::new(renderer).unwrap();
    let text_components = text_factory
        .create("ChatGPT Conversation (dogs)", (800f32, 600f32), 0.9, 1f32);
    world.extend(text_components);

    let file = env::args().skip(1).next().expect("Expected a file to be passed!");
    println!("file {}", file);

    //create the camera
    let camera = Camera::new(1600, 1200);

    world.push((camera,));
}

pub fn run() {
    env_logger::init();
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_inner_size(PhysicalSize::<u32> { width: 1600, height: 1200 })
        .build(&event_loop).unwrap();

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
                Err(wgpu::SurfaceError::Lost) => renderer.find_display(),
                Err(wgpu::SurfaceError::OutOfMemory) => *control_flow = ControlFlow::Exit,
                Err(e) => eprintln!("{:?}", e),
            }
        }
        Event::MainEventsCleared => {
            window.request_redraw();
        }
        _ => {}
    });
}
