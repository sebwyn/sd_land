use std::env;

use legion::{World, Entity, Schedule, Resources, system};
use winit::{
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder, dpi::PhysicalSize,
};

use crate::{
    background_renderer::BackgroundRenderer, 
    ui_box_renderer::UiBox, 
    colorscheme::hex_color, 
    layout::{Element, DemandedLayout, DemandValue, LayoutProvider, Anchor}, 
    text_renderer::TextBox, 
    ui_event_system::{UserEventListener, text_box_on_key_event}, 
    buffer_renderer::{BufferRenderer, BufferView},
    buffer::Buffer
};
use crate::background_renderer::add_render_background;
use crate::buffer_renderer::{add_render_buffers};
use crate::buffer_system::add_buffer_system;
use crate::event::{Event, InputState, to_user_event};
use crate::renderer::render_api::RenderApi;

pub struct EnttRef(pub Entity);

fn _theme_selector_view(world: &mut World) {
    let preview_box = UiBox { 
        color: hex_color("#FFFFFF").unwrap(),
        opacity: 0.2,
        corner_radius: 100f32,
        border_width: 8f32,
        border_color: [0f32, 0f32, 0f32],
        ..Default::default()
    };

    let preview_layout = Element::new(DemandedLayout { 
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

    let title_layout = Element::new(DemandedLayout {
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

    let explanation_layout = Element::new(DemandedLayout {
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

    let text_box_layout = Element::new(DemandedLayout {
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

    // systems.register_event_systems(buffer_on_event);

    // let ui_box_renderer = UiBoxRenderer::default();
    // let text_renderer = TextRenderer::new("assets/fonts/RobotoMono-VariableFont_wght.ttf").unwrap();
    // renderer.push_subrenderer(ui_box_renderer);
    // renderer.push_subrenderer(text_renderer);

    // systems.register_event_systems(ui_on_event);
    // systems.register_update_system(crate::layout::layout_on_update);
}

#[derive(PartialEq, Eq)]
pub enum Command {
    CloseApp
}

#[system]
fn begin_render(#[resource] render_api: &mut RenderApi, #[resource] commands: &mut Vec<Command>) {
    match render_api.begin_render() {
        Ok(_) => {}
        Err(wgpu::SurfaceError::Lost) => render_api.find_display(),
        Err(wgpu::SurfaceError::OutOfMemory) => commands.push(Command::CloseApp),
        Err(e) => eprintln!("{:?}", e),
    }
}

#[system]
fn end_render(#[resource] render_api: &mut RenderApi) {
    render_api.flush();
}

pub fn run() {
    env_logger::init();
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_inner_size(PhysicalSize::<u32> { width: 3200, height: 2400 })
        .build(&event_loop).unwrap();

    let mut renderer = RenderApi::new(&window);
    let mut world = World::default();

    let file_to_open = env::args().nth(1).unwrap_or("src/app.rs".to_string());

    let buffer = Buffer::load(&file_to_open).unwrap();
    let buffer_view = BufferView::new(200, 2800, 0, 2400)
        .font("assets/fonts/RobotoMono-VariableFont_wght.ttf")
        .line_height(50f32);

    world.push((buffer, buffer_view));

    let mut background_renderer = BackgroundRenderer::new("assets/castle.png", &mut renderer).unwrap();
    let mut buffer_renderer = BufferRenderer::new(&mut renderer);

    let mut resources = Resources::default();
    resources.insert(renderer);
    resources.insert(background_renderer);
    resources.insert(buffer_renderer);

    let events: Vec<Event> = Vec::new();
    let commands: Vec<Command> = Vec::new();

    resources.insert(events);
    resources.insert(commands);

    let mut schedule_builder = Schedule::builder();

    let mut input_state = InputState::default();

    add_buffer_system(&mut schedule_builder);

    schedule_builder.add_system(begin_render_system());
    add_render_background(&mut schedule_builder);
    add_render_buffers(&mut schedule_builder);
    schedule_builder.add_system(end_render_system());

    let mut schedule = schedule_builder.build();

    event_loop.run(move |event, _, control_flow| {
        let user_events = to_user_event(&event, &mut input_state);

        resources.get_mut::<Vec<Event>>().unwrap().extend(user_events);

        match event {
            winit::event::Event::WindowEvent {
                event: winit::event::WindowEvent::CloseRequested {},
                window_id,
            } if window_id == window.id() => *control_flow = ControlFlow::Exit,

            winit::event::Event::RedrawRequested(_) => {
                schedule.execute(&mut world, &mut resources);

                resources.get_mut::<Vec<Event>>().unwrap().clear();

                if resources.get::<Vec<Command>>().unwrap().contains(&Command::CloseApp) {
                    *control_flow = ControlFlow::Exit;
                }
            },
            winit::event::Event::MainEventsCleared => {
                window.request_redraw();
            },
            _ => {}
        }
    });
}