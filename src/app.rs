use legion::World;
use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

use crate::{renderer::Renderer, graphics::{RectangleBuilder, Vertex}, pipeline::Pipeline, shader_types::{Texture, Sampler}};

fn initialize_world(renderer: &mut Renderer, world: &mut World) {
    let pipeline = renderer.create_pipeline(Pipeline::load::<Vertex>(include_str!("shader.wgsl")).unwrap());

    let material = renderer.create_material(pipeline).unwrap();

    let texture = renderer.create_texture("src/happy-tree.png");
    if !renderer.update_material(material, "t_diffuse", Texture::new(texture)) { panic!("Failed to update t_diffuse!")}

    let sampler = renderer.create_sampler();
    renderer.update_material(material, "s_diffuse", Sampler::new(sampler));

    let rectangle = RectangleBuilder::default()
        .position(-0.5, -0.5)
        .size(0.5, 0.5)
        .build();

    let rectangle_2 = RectangleBuilder::default()
        .position(-0.0, -0.0)
        .size(0.5, 0.5)
        .build();

    world.push((rectangle, material));
    world.push((rectangle_2, material));
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
            WindowEvent::ScaleFactorChanged { new_inner_size, .. } => renderer.resize(**new_inner_size),
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
        },
        Event::MainEventsCleared => {
            window.request_redraw();
        },
        _ => {}
    });
}