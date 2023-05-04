use legion::{Entity, IntoQuery, World};
use winit::dpi::PhysicalPosition;

use crate::system::Event;

pub struct View {
    left: u32,
    right: u32,
    top: u32,
    bottom: u32,
    near: f32,
    far: f32,
}

pub struct ViewRef(pub Entity);

impl View {
    pub fn change_left(&mut self, left: u32) { self.left = left; }
    pub fn change_right(&mut self, right: u32) { self.right = right; }
    pub fn change_bottom(&mut self, bottom: u32) { self.bottom = bottom; }
    pub fn change_top(&mut self, top: u32) { self.top = top; }

    pub fn right(&self) -> u32 { self.right }
    pub fn top(&self) -> u32 { self.top }
    
    pub fn left(&self) -> u32 { self.left }
    pub fn bottom(&self) -> u32 { self.bottom }

    pub fn near(&self) -> f32 { self.near.clamp(0.0, 1.0) }
    pub fn far(&self) -> f32 { self.far.clamp(0.0, 1.0) }

    pub fn x_pos(&self) -> f32 { self.left as f32 }
    pub fn y_pos(&self) -> f32 { self.top as f32 }
    pub fn width(&self) -> f32 { (self.right - self.left) as f32}
    pub fn height(&self) -> f32 { (self.bottom - self.top) as f32 }

    pub fn contains_point(&self, point: &PhysicalPosition<f64>) -> bool {
        if self.left < point.x as u32 && self.right > point.x as u32 {
            if self.top < point.y as u32 && self.bottom > point.y as u32 {
                return true;
            }
        }
        false
    }
}

impl View {
    pub fn new(left: u32, right: u32, bottom: u32, top: u32, near: f32, far: f32) -> Self {
        Self {
            left,
            right,
            top,
            bottom,
            near,
            far
        }
    }
}

pub fn view_on_event(world: &mut World, event: &Event) {
    if let &Event::Resize(new_size) = event {
        let mut query = <&mut View>::query();

        for view in query.iter_mut(world) {
            // view.
        }
    }
}