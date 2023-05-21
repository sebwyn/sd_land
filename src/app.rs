use std::env;

use legion::{World, Entity};
use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder, dpi::PhysicalSize,
};

use crate::{
    renderer::render_api::Renderer,
    background_renderer::BackgroundRenderer, ui_box_renderer::{UiBox, UiBoxRenderer}, colorscheme::hex_color, layout::{Layout, DemandedLayout, DemandValue, Transform, LayoutProvider}, text_renderer::{TextBox, TextRenderer}, buffer_system::buffer_on_event, buffer_renderer::{BufferRenderer, BufferView}, buffer::Buffer
};

use crate::system::Systems;

pub struct EnttRef(pub Entity);

fn initialize_world(renderer: &mut Renderer, world: &mut World, systems: &mut Systems) {
    let preview_box = UiBox { color: hex_color("#FFFFFF").unwrap(), view: None, opacity: 0.2 };
    let preview_layout = Layout::new(DemandedLayout { 
        size: Some([DemandValue::Percent(0.8), DemandValue::Percent(1.0)]), 
        position: Some([DemandValue::Percent(0.1), DemandValue::Percent(0f32)]), 
        depth: Some(0.1), 
        visible: true,
        ..Default::default()
    }).child_layout_provider(LayoutProvider::Relative);

    let lorem_text = TextBox { 
        text: 
r#"Lorem ipsum dolor sit amet, consectetur adipiscing elit. 
Sed sit amet dolor id tellus placerat molestie. 
Proin in auctor elit, vitae volutpat orci. 
Duis dapibus luctus varius. 
Vestibulum ut dolor dui. 
Integer at molestie sapien, in hendrerit tellus. 
Etiam fringilla ligula at erat sodales, ut elementum lacus porta. 
Maecenas mollis leo purus, quis tincidunt odio dapibus sed. 
Suspendisse molestie eleifend risus. 
In dui erat, pharetra a cursus vel, laoreet id eros. 
Integer non risus eget sapien eleifend tempus. 
Sed fermentum efficitur ultrices. 
Praesent id varius nunc, quis placerat nisl. 
Duis a purus nec orci sollicitudin accumsan."#.to_string(),
       text_color: hex_color("#FFFFFF").unwrap(), 
       line_height: 50f32, 
       font_scale: 0.6
    };

    let lorem_text_layout = Layout::new(DemandedLayout { 
        size: Some([DemandValue::Percent(0.9), DemandValue::Percent(0.5)]),
        position: Some([DemandValue::Percent(0.05), DemandValue::Percent(0.4)]),
        depth: Some(0.5),
        visible: true,
        ..Default::default()
     }).parent(&preview_layout);

    world.push((Transform::default(), preview_box, preview_layout));
    world.push((Transform::default(), lorem_text, lorem_text_layout));

    let background_renderer = BackgroundRenderer::new("assets/castle.png").unwrap();
    let ui_box_renderer = UiBoxRenderer::default();
    let text_renderer = TextRenderer::new("Roboto Mono").unwrap();

    renderer.push_subrenderer(background_renderer);
    renderer.push_subrenderer(ui_box_renderer);
    renderer.push_subrenderer(text_renderer);
    
    systems.register_event_systems(buffer_on_event);

    // systems.register_event_systems(buffer_on_event);
    systems.register_update_system(crate::layout::layout_on_update);
}

pub fn run() {
    env_logger::init();
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_inner_size(PhysicalSize::<u32> { width: 3200, height: 2400 })
        // .with_decorations(false)
        .build(&event_loop).unwrap();

    let mut renderer = Renderer::new(&window);
    let mut world = World::default();

    let mut systems = Systems::new(window.inner_size());

    initialize_world(&mut renderer, &mut world, &mut systems);

    event_loop.run(move |event, _, control_flow| {
        systems.on_event(&mut world, &event);

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
                systems.update(&mut world);

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