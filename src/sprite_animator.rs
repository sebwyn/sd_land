use std::time::{Duration, Instant};
use legion::system;
use legion::systems::Builder;
use crate::sprite::SpriteSheetSprite;

#[derive(Clone)]
pub struct SpriteAnimation {
    frames: Vec<(Duration, (u32, u32))>,
    current_frame: usize,

    last_frame_time: Option<Instant>,
}

impl SpriteAnimation {
    pub fn new_constant_time(duration: Duration, frames: Vec<(u32, u32)>) -> Self {
        let timed_frames = frames.into_iter().map(|frame| (duration, frame)).collect();

        Self {
            frames: timed_frames,
            current_frame: 0,
            last_frame_time: None,
        }
    }
}

pub fn add_sprite_animation(schedule: &mut Builder) { schedule.add_system(animation_update_system()); }

#[system(for_each)]
fn animation_update(sprite: &mut SpriteSheetSprite, animation: &mut SpriteAnimation) {
    //if the animation hasn't been started, start it
    if let Some(last_frame_time) = animation.last_frame_time {
        let (duration, current_tile) = animation.frames[animation.current_frame];
        if last_frame_time.elapsed() > duration {
            sprite.set_tile(current_tile.0, current_tile.1);

            animation.current_frame += 1;
            animation.current_frame %= animation.frames.len();
            animation.last_frame_time = Some(Instant::now());
        }
    } else {
        animation.current_frame = 0;
        let (_, current_tile) = animation.frames[animation.current_frame];
        sprite.set_tile(current_tile.0, current_tile.1);
        animation.last_frame_time = Some(Instant::now());
    }
}