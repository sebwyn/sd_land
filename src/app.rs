use std::collections::HashMap;
use std::time::Duration;
use legion::{system, Resources, Schedule, World};
use winit::{
    dpi::PhysicalSize,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

use crate::event::{to_user_event, Event, InputState};
use crate::grid_renderer::{add_grid_lines_subrender, GridLines};
use crate::layout::Transform;
use crate::renderer::camera::Camera;
use crate::renderer::render_api::RenderApi;
use crate::scene_camera::add_scene_camera_controller;
use crate::sprite::{add_sprite_subrender, ActiveSceneCamera, SpriteRenderer, SpriteSheet, SpriteSheetSprite};
use crate::sprite_animator::{add_sprite_animation, SpriteAnimation};

#[derive(PartialEq, Eq)]
pub enum Command {
    CloseApp,
}

#[system]
fn update_screen_size(#[resource] screen_size: &mut (f32, f32), #[resource] events: &Vec<Event>) {
    events
        .iter()
        .find_map(|e| -> Option<PhysicalSize<u32>> {
            if let Event::Resize(new_size) = e {
                Some(*new_size)
            } else {
                None
            }
        })
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
        .with_inner_size(PhysicalSize::<u32> {
            width: 3200,
            height: 2400,
        })
        .build(&event_loop)
        .unwrap();

    let mut renderer = RenderApi::new(&window);
    let mut world = World::default();

    let mut schedule_builder = Schedule::builder();

    schedule_builder.add_system(update_screen_size_system());

    add_scene_camera_controller(&mut schedule_builder);

    add_sprite_animation(&mut schedule_builder);

    schedule_builder.add_system(begin_render_system());
    let grid_lines = GridLines::new(100f32, 100f32, [0.1, 0.1, 0.1], 2.5f32, &mut renderer);
    add_grid_lines_subrender(grid_lines, &mut schedule_builder);
    add_sprite_subrender(
        SpriteRenderer::new(&mut renderer).unwrap(),
        &mut schedule_builder,
    );
    schedule_builder.add_system(end_render_system());

    let mut schedule = schedule_builder.build();

    let mut resources = Resources::default();
    resources.insert(renderer);

    let events: Vec<Event> = Vec::new();
    let commands: Vec<Command> = Vec::new();

    resources.insert(events);
    resources.insert(commands);
    resources.insert((3200f32, 2400f32));

    let camera = Camera::new(800, 600);
    world.push((camera, ActiveSceneCamera));

    let mut sprite_sheets = HashMap::new();
    sprite_sheets.insert("Base Character", SpriteSheet::new("assets/sprites/simple_character/character/body.png", 8, 8));
    resources.insert(sprite_sheets);

    let walk_right_frames = (0..6).map(|i| (i, 6)).collect::<Vec<_>>();
    let walk_right_animation = SpriteAnimation::new_constant_time(
        Duration::from_millis(135),
        walk_right_frames);

    let walk_left_frames = (0..6).map(|i| (i, 7)).collect::<Vec<_>>();
    let walk_left_animation = SpriteAnimation::new_constant_time(
        Duration::from_millis(135),
        walk_left_frames.clone());

    let mut run_left_frames = walk_left_frames;
    let run_frame_times: Vec<Duration> = vec![80, 55, 125, 80, 55, 125].into_iter().map(Duration::from_millis).collect();

    run_left_frames[2].0 = 6;
    run_left_frames[5].0 = 7;

    let timed_frames = run_frame_times.into_iter().zip(run_left_frames.into_iter()).collect();

    let run_left_animation = SpriteAnimation::new(timed_frames);

    for x in 0..8 {
        for y in 0..8 {
            let animation = if x % 2 == 0 { &run_left_animation } else { &walk_right_animation }.clone();

            let mut sprite = SpriteSheetSprite::new("Base Character");

            let sprite_transform = Transform {
                size: (100.0, 100.0),
                position: (100.0 * x as f32, 100.0 * y as f32),
                depth: 0.5,
                visible: true,
            };

            sprite.set_tile(x, y);

            world.push((sprite, sprite_transform, animation));
        }
    }


    let mut input_state = InputState::default();
    event_loop.run(move |event, _, control_flow| {
        let user_events = to_user_event(&event, &mut input_state);

        resources
            .get_mut::<Vec<Event>>()
            .unwrap()
            .extend(user_events);

        match event {
            winit::event::Event::WindowEvent {
                event: winit::event::WindowEvent::CloseRequested {},
                window_id,
            } if window_id == window.id() => *control_flow = ControlFlow::Exit,

            winit::event::Event::RedrawRequested(_) => {
                schedule.execute(&mut world, &mut resources);

                resources.get_mut::<Vec<Event>>().unwrap().clear();

                if resources
                    .get::<Vec<Command>>()
                    .unwrap()
                    .contains(&Command::CloseApp)
                {
                    *control_flow = ControlFlow::Exit;
                }
            }
            winit::event::Event::MainEventsCleared => {
                window.request_redraw();
            }
            _ => {}
        }
    });
}
