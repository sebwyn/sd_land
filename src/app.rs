use legion::{World, Entity};
use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder, dpi::PhysicalSize,
};

use crate::{
    renderer::render_api::Renderer,
    background_renderer::BackgroundRenderer, ui_box_renderer::{UiBox, UiBoxRenderer}, colorscheme::hex_color, layout::{Layout, DemandedLayout, DemandValue, LayoutProvider, Anchor}, text_renderer::{TextBox, TextRenderer}, buffer_system::buffer_on_event, ui_event_system::{UserEventListener, text_box_on_key_event, ui_on_event}
};

use crate::system::Systems;

pub struct EnttRef(pub Entity);

fn initialize_world(renderer: &mut Renderer, world: &mut World, systems: &mut Systems) {
    let preview_box = UiBox { 
        color: hex_color("#FFFFFF").unwrap(),
        opacity: 0.2,
        corner_radius: 100f32,
        border_width: 8f32,
        border_color: [0f32, 0f32, 0f32],
        ..Default::default()
    };

    let preview_layout = Layout::new(DemandedLayout { 
        size: Some([
            DemandValue::Percent(0.8), 
            DemandValue::Percent(1.0)
        ]), 
        position: Some([
            DemandValue::Percent(0.1), 
            DemandValue::Percent(0f32)
        ]), 
        depth: Some(0.1), 
        visible: true,
        ..Default::default()
    }).child_layout_provider(LayoutProvider::Relative);

    let title_text = TextBox { 
        text: "Theme Picker".to_string(),
        text_color: hex_color("#FFFFFF").unwrap(), 
        line_height: 50f32,
        font_scale: 0.8
    };

    let title_layout = Layout::new(DemandedLayout {
        size: Some([
            DemandValue::Percent(0.25), 
            DemandValue::Percent(0.1)
        ]),
        position: Some([DemandValue::Absolute(0f32), DemandValue::Absolute(-50f32)]),
        parent_anchor: Some([Anchor::Center, Anchor::Max]),
        child_anchor: Some([Anchor::Center, Anchor::Max]),
        depth: Some(0.5),
        visible: true,
        ..Default::default()
     }).parent(&preview_layout);

     let explanation_text = TextBox { 
        text: "Enter the path to an image to create a theme from:".to_string(),
        text_color: hex_color("#FFFFFF").unwrap(), 
        line_height: 50f32,
        font_scale: 0.6
    };

    let explanation_layout = Layout::new(DemandedLayout {
        size: Some([
            DemandValue::Percent(1.0), 
            DemandValue::Percent(0.25)
        ]),
        position: Some([DemandValue::Absolute(50f32), DemandValue::Absolute(-150f32)]),
        parent_anchor: Some([Anchor::Min, Anchor::Max]),
        child_anchor: Some([Anchor::Min, Anchor::Max]),
        depth: Some(0.5),
        visible: true,
        ..Default::default()
    }).parent(&preview_layout);

    let text_box_background = UiBox { 
        color: hex_color("#222222").unwrap(),
        ..Default::default()
    };

    let text_box_layout = Layout::new(DemandedLayout {
        size: Some([DemandValue::Percent(0.8f32), DemandValue::Absolute(51f32)]),
        position: Some([
            DemandValue::Absolute(50f32),
            DemandValue::Absolute(-200f32),
        ]),
        depth: Some(0.5),
        parent_anchor: Some([Anchor::Min, Anchor::Max]),
        child_anchor: Some([Anchor::Min, Anchor::Max]),
        visible: true,
        ..Default::default()
    }).parent(&preview_layout);

    let text_box_text = TextBox {
        text: "".to_string(),
        text_color: hex_color("#FFFFFF").unwrap(),
        line_height: 50f32,
        font_scale: 0.6,
    };

    world.push((preview_box, preview_layout));
    world.push((title_text, title_layout));
    world.push((explanation_text, explanation_layout));
    world.push((text_box_background, text_box_layout.clone()));
    world.push((text_box_text, text_box_layout, UserEventListener { 
        on_key_event: Some(text_box_on_key_event), 
        ..Default::default()
    }));

    let background_renderer = BackgroundRenderer::new("assets/castle.png").unwrap();
    let ui_box_renderer = UiBoxRenderer::default();
    let text_renderer = TextRenderer::new("Roboto Mono").unwrap();

    renderer.push_subrenderer(background_renderer);
    renderer.push_subrenderer(ui_box_renderer);
    renderer.push_subrenderer(text_renderer);
    
    // systems.register_event_systems(buffer_on_event);

    systems.register_event_systems(ui_on_event);
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