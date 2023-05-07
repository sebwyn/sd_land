use std::default;

use legion::World;
use winit::{dpi::{PhysicalSize, PhysicalPosition}, event::{ModifiersState, MouseButton}};

pub trait System {
    fn init(systems: &mut Systems);
}

pub struct Systems {
    event_systems: Vec<fn(&mut World, &Event)>,

    key_modifiers: ModifiersState,
    mouse_position: PhysicalPosition<f64>,
}

#[derive(Debug)]
pub enum Key {
    Char(char, Option<char>),
    Escape,
    Return,
    Left,
    Up,
    Right,
    Down,
    Tab,
    Backspace,
}

impl Key {
    fn char(c: char) -> Self {
        let uppercase = c.to_uppercase().next();
        if uppercase.is_none() {
            return Self::Char(c, None)
        }

        let uppercase = uppercase.unwrap();
        if uppercase != c {
            Self::Char(c, Some(uppercase))
        } else {
            let uppercase = 
            match c {
                '\\' => Some('|'),
                '\'' => Some('"'),
                ';' => Some(':'),
                ',' => Some('<'),
                '`' => Some('~'),
                '[' => Some('{'),
                '-' => Some('_'),
                '.' => Some('>'),
                ']' => Some('}'),
                '/' => Some('?'),
                '=' => Some('+'),


                '0' => Some(')'),
                '1' => Some('!'),
                '2' => Some('@'),
                '3' => Some('#'),
                '4' => Some('$'),
                '5' => Some('%'),
                '6' => Some('^'),
                '7' => Some('&'),
                '8' => Some('*'),
                '9' => Some('('),

                _ => None
            };

            Self::Char(c, uppercase)   
        }
    }
}

#[derive(Debug)]
pub enum Event {
    Resize(PhysicalSize<u32>),
    MouseScroll(PhysicalPosition<f64>, PhysicalPosition<f64>),
    KeyPress(Key, ModifiersState),
    KeyRelease(Key, ModifiersState),
    MousePress(MouseButton, PhysicalPosition<f64>),
    PrepareRender,
}

impl Systems {
    pub fn new() -> Self {
        Self {
            event_systems: Vec::new(),
            key_modifiers: ModifiersState::default(),
            mouse_position: PhysicalPosition::<f64>::default()
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

    pub fn prepare_render(&mut self, world: &mut World) {
        self.notify_event_systems(world, Event::PrepareRender);
    }

    pub fn update<T>(&mut self, world: &mut World, event: &winit::event::Event<T>) {
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
                            let mouse_scroll = Event::MouseScroll(*delta, self.mouse_position);
                            self.notify_event_systems(world, mouse_scroll)
                        }
                    },
                    winit::event::WindowEvent::KeyboardInput { input, ..  } => {
                        if let Some(key_code) = input.virtual_keycode {
                            //do some bullshit to shorten converting the key codes
                            let key = 
                            if (key_code as u8) < 10u8 {
                                Key::char((48 + (key_code as u8 + 1) % 10) as char)
                            } else if (key_code as u8) < 36 {
                                Key::char((87 + (key_code as u8)) as char)
                            } else {
                                match key_code {
                                    winit::event::VirtualKeyCode::Escape => Key::Escape,
                                    winit::event::VirtualKeyCode::Return => Key::Return,
                                    winit::event::VirtualKeyCode::Left => Key::Left,
                                    winit::event::VirtualKeyCode::Up => Key::Up,
                                    winit::event::VirtualKeyCode::Right => Key::Right,
                                    winit::event::VirtualKeyCode::Down => Key::Down,
                                    winit::event::VirtualKeyCode::Tab => Key::Tab,
                                    // winit::event::VirtualKeyCode::Delete => Key::Backspace,
                                    winit::event::VirtualKeyCode::Back => Key::Backspace,
                                    
                                    winit::event::VirtualKeyCode::Space => Key::char(' '),
                                    winit::event::VirtualKeyCode::Caret => Key::char('^'),
                                    winit::event::VirtualKeyCode::Apostrophe => Key::char('\''),
                                    winit::event::VirtualKeyCode::Asterisk => Key::char('*'),
                                    winit::event::VirtualKeyCode::At => Key::char('@'),
                                    winit::event::VirtualKeyCode::Backslash => Key::char('\\'),
                                    winit::event::VirtualKeyCode::Colon => Key::char(':'),
                                    winit::event::VirtualKeyCode::Comma => Key::char(','),
                                    winit::event::VirtualKeyCode::Equals => Key::char('='),
                                    winit::event::VirtualKeyCode::Grave => Key::char('`'),
                                    winit::event::VirtualKeyCode::LBracket => Key::char('['),
                                    winit::event::VirtualKeyCode::Minus => Key::char('-'),
                                    winit::event::VirtualKeyCode::Period => Key::char('.'),
                                    winit::event::VirtualKeyCode::Plus => Key::char('+'),
                                    winit::event::VirtualKeyCode::RBracket => Key::char(']'),
                                    winit::event::VirtualKeyCode::Semicolon => Key::char(';'),
                                    winit::event::VirtualKeyCode::Slash => Key::char('/'),
                                    _ => return
                                }
                            };

                            let event = 
                            match input.state {
                                winit::event::ElementState::Pressed => Event::KeyPress(key, self.key_modifiers),
                                winit::event::ElementState::Released => Event::KeyRelease(key, self.key_modifiers),
                            };
                            self.notify_event_systems(world, event);
                        }
                    },
                    winit::event::WindowEvent::MouseInput { state, button, .. } => {
                        match state {
                            winit::event::ElementState::Pressed => {
                                let event = Event::MousePress(*button, self.mouse_position);
                                self.notify_event_systems(world, event);
                            },
                            _ => {}
                        }
                    }
                    winit::event::WindowEvent::ModifiersChanged(modifiers) => {
                        self.key_modifiers = *modifiers;
                    },
                    winit::event::WindowEvent::CursorMoved { position, .. } => {
                        self.mouse_position = *position;
                    },
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
