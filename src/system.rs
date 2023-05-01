use legion::World;
use winit::dpi::{PhysicalSize, PhysicalPosition};

pub trait System {
    fn init(systems: &mut Systems);
}

pub struct Systems {
    event_systems: Vec<fn(&mut World, &Event)>
}

pub enum Event {
    Resize(PhysicalSize<u32>),
    MouseScroll(PhysicalPosition<f64>)
}

impl Systems {
    pub fn new() -> Self {
        Self {
            event_systems: Vec::new()
        }
    }

    pub fn register_event_systems(&mut self, notify: fn(&mut World, &Event))
    {
        self.event_systems.push(notify);
    }

    fn notify_event_systems(&self, world: &mut World, event: Event)
    {
        for event_system in self.event_systems.iter() {
            event_system(world, &event);
        }
    }

    pub fn update<T>(&self, world: &mut World, event: &winit::event::Event<T>) {
        match event {
            winit::event::Event::WindowEvent { event, .. } => {
                match event {
                    winit::event::WindowEvent::Resized(new_size) => {
                        let resize = Event::Resize(*new_size);
                        self.notify_event_systems(world, resize);
                    },
                    winit::event::WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                        let resize = Event::Resize(**new_inner_size);
                        self.notify_event_systems(world, resize);
                    },
                    winit::event::WindowEvent::MouseWheel { delta, .. } => {
                        if let winit::event::MouseScrollDelta::PixelDelta(delta) = delta {
                            // println!("Mouse scroll: {:?}, phase: {:?}", delta, phase);
                            let mouse_scroll = Event::MouseScroll(*delta);
                            self.notify_event_systems(world, mouse_scroll)
                        }
                    }
                    _ => {}
                }
            },
            _ => {}
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Resize(pub winit::dpi::PhysicalSize<u32>);

pub trait EventListener<T> {
    fn notify(&self, event: T, world: &mut World);
}
