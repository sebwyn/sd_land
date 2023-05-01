use std::{env, fs::File, io::Read};

use legion::World;
use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder, dpi::PhysicalSize,
};

use crate::{
    renderer::Renderer, 
    text::TextBoxFactory, camera::{Camera, camera_on_event}, system::Systems, ui_box::UiBoxFactory, buffer::render_code
};

fn initialize_world(renderer: &mut Renderer, world: &mut World) {
    let ui_box_factory = UiBoxFactory::new(renderer).unwrap();
    let text_factory = TextBoxFactory::new(renderer, "Roboto Mono").unwrap();

    let file = env::args().skip(1).next().expect("Expected a file to be passed!");
    println!("file {}", file);

    let mut text = String::new();
    let mut file = File::open(file).expect("File does not exit!");
    file.read_to_string(&mut text).expect("Failed to read to string");

    render_code(&text, world, &text_factory, &ui_box_factory);

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

    let mut systems = Systems::new();
    systems.register_event_systems(camera_on_event);

    // window.set_decorations(false);

    initialize_world(&mut renderer, &mut world);

    event_loop.run(move |event, _, control_flow| {
        systems.update(&mut world, &event);

        match event {
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == window.id() => match event {
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

                //game events
                WindowEvent::Resized(new_size) => { 
                    renderer.resize(*new_size) 
                },
                WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                    renderer.resize(**new_inner_size)
                }
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
        }
    });
}
