use std::time::{Duration, Instant};
use legion::{component, system};
use legion::systems::Builder;
use crate::layout::Transform;
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

    pub fn new(timed_frames: Vec<(Duration, (u32, u32))>) -> Self {
        Self {
            frames: timed_frames,
            current_frame: 0,
            last_frame_time: None,
        }
    }
}

pub fn add_sprite_animation(schedule: &mut Builder) { schedule.add_system(animation_update_system()); }


//only update animations that are actually shown on the screen
#[system(for_each)]
#[filter(component::< Transform > ())]
fn animation_update(sprite: &mut SpriteSheetSprite, animation: &mut SpriteAnimation) {
    //if the animation hasn't been started, start it
    if let Some(last_frame_time) = animation.last_frame_time {
        let (duration, _) = animation.frames[animation.current_frame];
        if last_frame_time.elapsed() > duration {
            animation.current_frame += 1;
            animation.current_frame %= animation.frames.len();
            animation.last_frame_time = Some(Instant::now());
            let (_, current_tile) = animation.frames[animation.current_frame];


            sprite.set_tile(current_tile.0, current_tile.1);
        }
    } else {
        animation.current_frame = 0;
        let (_, current_tile) = animation.frames[animation.current_frame];
        sprite.set_tile(current_tile.0, current_tile.1);
        animation.last_frame_time = Some(Instant::now());
    }
}