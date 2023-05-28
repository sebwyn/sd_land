use legion::system;
use legion::systems::Builder;
use winit::event::MouseButton;
use crate::{buffer_renderer::BufferView, buffer::{Buffer, BufferRange}};
use crate::event::{Event, Key, MouseDrag};

#[derive(Clone, Copy)]
pub struct Cursor(pub usize, pub usize);

pub fn add_buffer_system(schedule: &mut Builder) { schedule.add_system(buffer_on_event_system()); }

#[system(for_each)]
pub fn buffer_on_event(buffer: &mut Buffer, buffer_view: &mut BufferView, #[resource] events: &Vec<Event>) {
    for event in events {
        match event {
            Event::KeyPress(key, modifiers) if !modifiers.logo() && !modifiers.alt() && !modifiers.ctrl() => {
                let character = match key {
                    Key::Char(_, uppercase) if modifiers.shift() && uppercase.is_some() => Some(uppercase.unwrap()),
                    Key::Char(lowercase, _) if !modifiers.shift() => Some(*lowercase),
                    _ => None
                };
                if let Some(character) = character {
                    buffer.insert_character(character);
                } else {
                    match key {
                        Key::Backspace => buffer.delete(),
                        Key::Return => buffer.insert_newline(),
                        Key::Tab => buffer.insert_string("    "),
                        Key::Left => buffer.move_left(modifiers.shift()),
                        Key::Right => buffer.move_right(modifiers.shift()),
                        Key::Up => buffer.move_up(modifiers.shift()),
                        Key::Down => buffer.move_down(modifiers.shift()),
                        _ => {}
                    }
                }
            },
            Event::KeyPress(key, modifiers) if modifiers.logo() && !modifiers.shift() && !modifiers.alt() && !modifiers.ctrl() => {
                if matches!(key, Key::Char(s, ..) if *s == 's') {
                    buffer.save();
                }
            },
            Event::KeyPress(key, modifiers) if modifiers.alt() && !modifiers.ctrl() && !modifiers.logo() => {
                match key {
                    Key::Right => buffer.move_forward_word(modifiers.shift()),
                    Key::Left => buffer.move_backward_word(modifiers.shift()),
                    _ => {}
                }
            },
            Event::MouseScroll(scroll, position, _) if buffer_view.contains(position) => {
                buffer_view.scroll_vertically(scroll.y as f32);
            },
            Event::MouseClick(MouseButton::Left, position, ..) => {
                if let Some((row, col)) = buffer_view.buffer_position(buffer, position) {
                    buffer.cursor = Cursor(row, col);
                    buffer.selection = None;
                }
            },
            Event::MouseDrag(MouseDrag {
                 button: MouseButton::Left,
                 start,
                 current_position,
                 ..
            }) => {
                if let Some(start_buffer_position) = buffer_view.buffer_position(buffer, start) {
                    if let Some(end_buffer_position) = buffer_view.buffer_position(buffer, current_position) {
                        buffer.selection = None;
                        buffer.selection = Some(BufferRange::new(start_buffer_position, end_buffer_position));

                        buffer.cursor = Cursor(end_buffer_position.0, end_buffer_position.1);
                    }
                }
            },
            _ => {}
        }
    }
}