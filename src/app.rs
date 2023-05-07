use std::env;

use legion::{World, Entity};
use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder, dpi::PhysicalSize,
};

use crate::{
    renderer::Renderer, camera::Camera, view::{View, ViewRef}, buffer::{Buffer, buffer_on_event, ColorScheme}, system::Systems, graphics::{Visible, Vertex}, shortcuts::trigger_shortcuts, text::prepare_font, cursor::{cursor_on_event, Cursor}
};

pub struct EnttRef(pub Entity);

fn initialize_world(renderer: &mut Renderer, world: &mut World, systems: &mut Systems) {
    let (text_material, font) = prepare_font(renderer, "Roboto Mono").unwrap();

    let file = env::args().skip(1).next().expect("Expected a file to be passed!");
    println!("file {}", file);

    //create the camera
    let camera = Camera::new(3200, 2400);
    //create a view
    let view = View::new(0, 3200, 2400, 0, -100.0, 100.0);
    let view_entity = world.push((view, camera, Visible));

    //create a buffer
    let buffer = Buffer::load(
        &file, 
        50f32, 
        ColorScheme::default(), 
        font.clone(),
        0.6f32,
    ).unwrap();

    let buffer_entity = world.push((buffer, text_material, ViewRef(view_entity), Vec::<Vertex>::new()));

    let cursor = Cursor::new(buffer_entity, view_entity);

    world.push((cursor,));

    //create the shortcuts
    systems.register_event_systems(buffer_on_event);
    systems.register_event_systems(trigger_shortcuts);
    systems.register_event_systems(cursor_on_event)
}

pub fn run() {
    env_logger::init();
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_inner_size(PhysicalSize::<u32> { width: 3200, height: 2400 })
        .build(&event_loop).unwrap();

    let mut renderer = Renderer::new(&window);
    let mut world = World::default();

    let mut systems = Systems::new();

    initialize_world(&mut renderer, &mut world, &mut systems);

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
                systems.prepare_render(&mut world);

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
