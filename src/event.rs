use std::collections::HashMap;
use bitflags::bitflags;
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::{KeyboardInput, ModifiersState, MouseButton};

#[derive(Debug, Clone, Copy)]
pub enum Event {
    Resize(PhysicalSize<u32>),
    MouseScroll(PhysicalPosition<f64>, PhysicalPosition<f64>, ModifiersState),
    KeyPress(Key, ModifiersState),
    KeyRelease(Key, ModifiersState),
    MousePress(MouseButton, PhysicalPosition<f64>, ModifiersState),
    MouseMoved(MouseState, PhysicalPosition<f64>, ModifiersState),
    MouseRelease(MouseButton, PhysicalPosition<f64>, ModifiersState),
    MouseDrag(MouseDrag),
    MouseClick(MouseButton, PhysicalPosition<f64>, ModifiersState),
}

#[derive(Default)]
pub struct InputState {
    modifiers: ModifiersState,
    mouse_state: MouseState,
    mouse_position: PhysicalPosition<f64>,
    drags: HashMap<MouseButton, MouseDrag>,
}

bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct MouseState: u32 {
        const LEFT = 0b00000001;
        const RIGHT = 0b00000010;
        const MIDDLE = 0b00000100;
    }
}

impl From<&MouseButton> for MouseState {
    fn from(value: &MouseButton) -> Self {
        match value {
            MouseButton::Left => MouseState::LEFT,
            MouseButton::Right => MouseState::RIGHT,
            MouseButton::Middle => MouseState::MIDDLE,
            MouseButton::Other(_) => MouseState::empty(),
        }
    }
}

impl Default for MouseState {
    fn default() -> Self {
        Self::empty()
    }
}

#[derive(Debug, Clone, Copy)]
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

#[derive(Debug, Clone, Copy)]
pub struct MouseDrag {
    pub start: PhysicalPosition<f64>,
    pub current_position: PhysicalPosition<f64>,
    pub button: MouseButton,
    pub finish: Option<PhysicalPosition<f64>>,
}

pub fn to_user_event<T>(event: &winit::event::Event<T>, input_state: &mut InputState) -> Vec<Event> {
    if let winit::event::Event::WindowEvent { event, .. } = event {
        let mut events = Vec::new();

        match event {
            winit::event::WindowEvent::Resized(new_size) => {
                events.push(Event::Resize(*new_size));
            },
            winit::event::WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                events.push(Event::Resize(**new_inner_size));
            },
            winit::event::WindowEvent::MouseWheel {
                delta: winit::event::MouseScrollDelta::PixelDelta( delta),
                ..
            } => {
                events.push(Event::MouseScroll(*delta, input_state.mouse_position, input_state.modifiers));
            },
            winit::event::WindowEvent::KeyboardInput { input: KeyboardInput {
                state,
                virtual_keycode: Some(key_code), .. }, ..
            } => {
                    let key =
                        if (*key_code as u8) < 10u8 {
                            Key::char((48 + (*key_code as u8 + 1) % 10) as char)
                        } else if (*key_code as u8) < 36 {
                            Key::char((87 + (*key_code as u8)) as char)
                        } else {
                            match key_code {
                                winit::event::VirtualKeyCode::Escape => Key::Escape,
                                winit::event::VirtualKeyCode::Return => Key::Return,
                                winit::event::VirtualKeyCode::Left => Key::Left,
                                winit::event::VirtualKeyCode::Up => Key::Up,
                                winit::event::VirtualKeyCode::Right => Key::Right,
                                winit::event::VirtualKeyCode::Down => Key::Down,
                                winit::event::VirtualKeyCode::Tab => Key::Tab,
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
                                _ => return Vec::new()
                            }
                        };

                let e = match state {
                    winit::event::ElementState::Pressed => Event::KeyPress(key, input_state.modifiers),
                    winit::event::ElementState::Released => Event::KeyRelease(key, input_state.modifiers),
                };

                events.push(e);
            },
            winit::event::WindowEvent::MouseInput {
                state: winit::event::ElementState::Pressed,
                button,
                ..
            } => {
                input_state.mouse_state |= MouseState::from(button);

                input_state.drags.insert(*button,
                                  MouseDrag {
                                      start: input_state.mouse_position,
                                      current_position: input_state.mouse_position,
                                      button: *button,
                                      finish: None
                                  });

                events.push(Event::MousePress(*button, input_state.mouse_position, input_state.modifiers))
            }
            winit::event::WindowEvent::MouseInput {
                state: winit::event::ElementState::Released,
                button, ..
            } => {
                input_state.mouse_state &= MouseState::from(button).complement();

                let mut drag = input_state.drags.remove(button).unwrap();

                if same_position(drag.start, input_state.mouse_position) {
                    events.push(Event::MouseClick(*button, input_state.mouse_position, input_state.modifiers));
                } else {
                    drag.current_position = input_state.mouse_position;
                    drag.finish = Some(input_state.mouse_position);
                    events.push(Event::MouseDrag(drag))
                }

                events.push(Event::MouseRelease(*button, input_state.mouse_position, input_state.modifiers))
            },
            winit::event::WindowEvent::CursorMoved { position, .. } => {
                input_state.mouse_position = *position;

                for (_, drag) in input_state.drags.iter() {
                    let mut drag = *drag;

                    if !same_position(drag.current_position, input_state.mouse_position) {
                        drag.current_position = input_state.mouse_position;

                        events.push(Event::MouseDrag(drag))
                    }
                }

                events.push(Event::MouseMoved(
                    input_state.mouse_state,
                    input_state.mouse_position,
                    input_state.modifiers
                ));
            },
            winit::event::WindowEvent::ModifiersChanged(modifiers) => {
                input_state.modifiers = *modifiers;
            },
            _ => {}
        }

        events
    } else {
        Vec::new()
    }
}

fn same_position(a: PhysicalPosition<f64>, b: PhysicalPosition<f64>) -> bool {
    a.x - 2.5 < b.x && b.x < a.x + 2.5 &&
        a.y - 2.5 < b.y && b.y < a.y + 2.5
}