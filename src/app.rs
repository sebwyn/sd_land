use std::env;

use legion::{World, Entity};
use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder, dpi::PhysicalSize,
};

use crate::{
    renderer::Renderer, camera::{Camera, camera_on_event}, view::View, buffer::Buffer, system::Systems, graphics::Visible, file_searcher::emplace_find_menu, text::TextBoxFactory, ui_box::UiBoxFactory, shortcuts::trigger_shortcuts
};

pub struct EnttRef(pub Entity);

fn initialize_world(renderer: &mut Renderer, world: &mut World, systems: &mut Systems) {
    let text_factory = TextBoxFactory::new(renderer, "Roboto Mono").unwrap();
    let ui_box_factory = UiBoxFactory::new(renderer).unwrap();

    let file = env::args().skip(1).next().expect("Expected a file to be passed!");
    println!("file {}", file);

    //create the camera
    let camera = Camera::new(1600, 2400);
    //create a view
    let view = View::new(0, 1600, 2400, 0, -100.0, 100.0);
    let view_entity = world.push((view, camera));

    world.entry(view_entity).unwrap().add_component(Visible);

    //create a buffer
    let buffer = Buffer::load(&file).unwrap();

    buffer.emplace_in_view(renderer, world, view_entity, None, 50f32, 0.65, "Roboto Mono");

    let file_searcher = emplace_find_menu(world, &text_factory, &ui_box_factory).unwrap();
    //add an entity ref onto this entity
    world.entry(file_searcher).unwrap()
        .add_component(EnttRef(file_searcher));

    //create the shortcuts
    systems.register_event_systems(camera_on_event);
    systems.register_event_systems(trigger_shortcuts);

    // let ui_box = 
    //     UiBoxFactory::new(renderer).unwrap().create("#FFFFFF", (0f32, 0f32), (400f32, 300f32), 0.5)
    //     .unwrap();

    // world.push(ui_box);
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

    // window.set_decorations(false);

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
