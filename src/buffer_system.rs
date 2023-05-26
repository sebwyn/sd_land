use legion::{World, IntoQuery};
use winit::event::MouseButton;
use crate::{system::{Event, MouseDrag, Key, Systems}, buffer_renderer::BufferView, buffer::{Buffer, BufferRange}};

#[derive(Clone, Copy)]
pub struct Cursor(pub usize, pub usize);


pub fn buffer_on_event(event: &Event, world: &mut World, _: &Systems) {
    match event {
        Event::KeyPress(key, modifiers) if !modifiers.logo() && !modifiers.alt() && !modifiers.ctrl() => {
            let character = match key {
                Key::Char(_, uppercase) if modifiers.shift() && uppercase.is_some() => Some(uppercase.unwrap()),
                Key::Char(lowercase, _) if !modifiers.shift() => Some(*lowercase),
                _ => None
            };
            if let Some(character) = character {
                for buffer in <&mut Buffer>::query().iter_mut(world) {
                    buffer.insert_character(character);
                }
            } else {
                match key {
                    Key::Backspace => for buffer in <&mut Buffer>::query().iter_mut(world) {
                        buffer.delete();
                    },
                    Key::Return => for buffer in <&mut Buffer>::query().iter_mut(world) {
                        buffer.insert_newline();
                    },
                    Key::Tab => for buffer in <&mut Buffer>::query().iter_mut(world) {
                        buffer.insert_string("    ");
                    },
                    Key::Left => for buffer in <&mut Buffer>::query().iter_mut(world) {
                        buffer.move_left(modifiers.shift());
                    },
                    Key::Right => for buffer in <&mut Buffer>::query().iter_mut(world) {
                       buffer.move_right(modifiers.shift());
                    },
                    Key::Up => for buffer in <&mut Buffer>::query().iter_mut(world) {
                        buffer.move_up(modifiers.shift());
                    },
                    Key::Down => for buffer in <&mut Buffer>::query().iter_mut(world) {
                        buffer.move_down(modifiers.shift());
                    },
                    _ => {}
                }
            }
        },
        Event::KeyPress(key, modifiers) if modifiers.logo() && !modifiers.shift() && !modifiers.alt() && !modifiers.ctrl() => {
                match key {
                    Key::Char(s, ..) if *s == 's' => {
                        let mut query = <&Buffer>::query();
                        for buffer in query.iter(world) {
                            buffer.save();
                        }
                    }
                    _ => {}
                }
        },
        Event::KeyPress(key, modifiers) if modifiers.alt() && !modifiers.ctrl() && !modifiers.logo() => {
            match key {
                Key::Right => for buffer in <&mut Buffer>::query().iter_mut(world) {
                    buffer.move_forward_word(modifiers.shift());
                },
                Key::Left => for buffer in <&mut Buffer>::query().iter_mut(world) {
                    buffer.move_backward_word(modifiers.shift());
                },
                _ => {}
            }
        },
        Event::MouseScroll(scroll, position, _) => {
            for buffer_view in <&mut BufferView>::query().iter_mut(world) {
                if buffer_view.contains(position) {
                    buffer_view.scroll_vertically(scroll.y as f32);
                }
            }
        },
        Event::MouseClick(MouseButton::Left, position, ..) => {
            for (buffer, buffer_view) in <(&mut Buffer, &BufferView)>::query().iter_mut(world) {
                if let Some((row, col)) = buffer_view.buffer_position(buffer, position) {
                    buffer.cursor = Cursor(row, col);
                    buffer.selection = None;
                }
            }
        },
        Event::MouseDrag(MouseDrag { 
            button: MouseButton::Left, 
            start, 
            current_position, 
            .. 
        }) => {  
            for (buffer, buffer_view) in <(&mut Buffer, &BufferView)>::query().iter_mut(world) {
                if let Some(start_buffer_position) = buffer_view.buffer_position(buffer, start) {
                    if let Some(end_buffer_position) = buffer_view.buffer_position(buffer, current_position){
                        buffer.selection = None;
                        buffer.selection = Some(BufferRange::new(start_buffer_position, end_buffer_position));
                        
                        buffer.cursor = Cursor(end_buffer_position.0, end_buffer_position.1);
                    }
                }
            }
        },
        _ => {}
    }
}