use std::env;

use legion::{World, Schedule, Resources, system};
use winit::{
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder, dpi::PhysicalSize,
};

use crate::{
    background_renderer::BackgroundRenderer,
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
use crate::layout::Transform;
use crate::renderer::camera::Camera;
use crate::renderer::render_api::RenderApi;
use crate::scene_camera::add_scene_camera_controller;
use crate::sprite_renderer::{ActiveSceneCamera, add_sprite_subrender, Sprite, SpriteRenderer};


#[derive(PartialEq, Eq)]
pub enum Command {
    CloseApp
}

#[system]
fn update_screen_size(#[resource] screen_size: &mut (f32, f32), #[resource] events: &Vec<Event>) {
    events.iter().find_map(|e| -> Option<PhysicalSize<u32>> {
        if let Event::Resize(new_size) = e {
            Some(*new_size)
        } else {
            None
        }})
        .and_then(|new_size| -> Option<()> {
            *screen_size = (new_size.width as f32, new_size.height as f32);
            None
        });
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

    let camera = Camera::new(3200, 2400);
    world.push((camera, ActiveSceneCamera));

    let sprite = Sprite {
        image_path: "assets/castle.png".to_string(),
        tex_origin: (0.0, 0.0),
        tex_dimensions: (1.0, 1.0),
    };

    let sprite_transform = Transform {
        size: (100.0, 100.0),
        position: (0.0, 0.0),
        depth: 0.5,
        visible: true,
    };

    world.push((sprite, sprite_transform));

    let mut schedule_builder = Schedule::builder();

    schedule_builder.add_system(update_screen_size_system());

    add_scene_camera_controller(&mut schedule_builder);

    schedule_builder.add_system(begin_render_system());
    add_sprite_subrender(SpriteRenderer::new(&mut renderer).unwrap(), &mut schedule_builder);
    schedule_builder.add_system(end_render_system());

    let mut schedule = schedule_builder.build();

    let mut resources = Resources::default();
    resources.insert(renderer);

    let events: Vec<Event> = Vec::new();
    let commands: Vec<Command> = Vec::new();

    resources.insert(events);
    resources.insert(commands);
    resources.insert((3200f32, 2400f32));

    let mut input_state = InputState::default();
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