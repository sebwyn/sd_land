use std::env;

use legion::{World, Entity};
use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder, dpi::PhysicalSize,
};

use crate::{
    renderer::render_api::Renderer, 
    buffer_system::buffer_on_event, 
    buffer_renderer::{BufferRenderer, BufferView}, 
    background_renderer::BackgroundRenderer
};

use crate::{buffer::Buffer, system::Systems};

pub struct EnttRef(pub Entity);

fn initialize_world(renderer: &mut Renderer, world: &mut World, systems: &mut Systems) {
    let buffer_renderer = BufferRenderer::default();

    let background_renderer = BackgroundRenderer::new("/Users/swyngaard/Documents/images/galactic.png")
        .unwrap();

    let file = env::args().nth(1).expect("Expected a file to be passed!");
    println!("file {}", file);

    let buffer_view = BufferView::new(200, 2600, 0, 3200)
        .font("Roboto Mono")
        .line_height(45f32)
        .font_scale(0.5);

    let buffer = Buffer::load(&file).unwrap();

    world.push((
        buffer,
        buffer_view
    ));

    renderer.push_subrenderer(background_renderer);
    renderer.push_subrenderer(buffer_renderer);
    //create the shortcuts
    systems.register_event_systems(buffer_on_event);
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
                match renderer.render(&world) {
                    Ok(_) => {}
                    Err(wgpu::SurfaceError::Lost) => renderer.find_display(),
                    Err(wgpu::SurfaceError::OutOfMemory) => *control_flow = ControlFlow::Exit,
                    Err(e) => eprintln!("{:?}", e),
                }
            },
            Event::MainEventsCleared => {
                window.request_redraw();
            },
            _ => {}
        }
    });
}