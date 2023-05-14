use legion::{World, IntoQuery};


use crate::{system::{Event}, buffer_renderer::BufferView};

#[derive(Clone, Copy)]
pub struct Cursor(pub usize, pub usize);


pub fn buffer_on_event(world: &mut World, event: &Event) {
    match event {
        /*Event::KeyPress(key, modifiers) => {
            if modifiers.logo() && !modifiers.shift() && !modifiers.alt() && !modifiers.ctrl() {
                match key {
                    Key::Char(s, ..) if *s == 's' => {
                        let mut query = <&Buffer>::query();
                        for buffer in query.iter(world) {
                            buffer.save();
                        }
                    },
                    Key::Char(s, ..) if *s == 'p' => for buffer in <&mut Buffer>::query().iter_mut(world) {
                        let positions = buffer.cursors.iter().map(|c| c.position).collect::<Vec<_>>();
                        
                        for (i, position) in positions.iter().enumerate() {
                            let new_position = buffer
                                .insert_str_at("\nprintln!(\"Hello, World!\")\n", *position);
                            
                            buffer.cursors[i].position = new_position;
                        }
                    }
                    _ => {}
                }
            } else if modifiers.alt() && !modifiers.ctrl() && !modifiers.logo()  {
                match key {
                    Key::Right => for buffer in <&mut Buffer>::query().iter_mut(world) {
                        for i in 0..buffer.cursors.len() {
                            buffer.cursors[i] = buffer.move_forward_word(buffer.cursors[i]);
                        }
                    },
                    Key::Left => for buffer in <&mut Buffer>::query().iter_mut(world) {
                        for i in 0..buffer.cursors.len() {
                            buffer.cursors[i] = buffer.move_backward_word(buffer.cursors[i]);
                        }
                    },
                    _ => {}
                }
            } 
            else if !modifiers.logo() && !modifiers.alt() {
                let character = match key {
                    Key::Char(_, uppercase) if modifiers.shift() && uppercase.is_some() => Some(uppercase.unwrap()),
                    Key::Char(lowercase, _) if !modifiers.shift() => Some(*lowercase),
                    _ => None
                };
                if let Some(character) = character {
                    for buffer in <&mut Buffer>::query().iter_mut(world) {
                        let positions = buffer.cursors.iter().map(|c| c.position).collect::<Vec<_>>();
    
                        for position in positions {
                            buffer.insert_at(character, position);
                        }
    
                        for i in 0..buffer.cursors.len() {
                            buffer.cursors[i].position = buffer.move_right(buffer.cursors[i].position)
                        }
                    }
                } else {
                    match key {
                        Key::Backspace => for buffer in <&mut Buffer>::query().iter_mut(world) {
                            let positions = buffer.cursors.iter().map(|c| c.position).collect::<Vec<_>>();
                            
                            for (i, position) in positions.iter().enumerate() {
                                buffer.cursors[i].position = buffer.move_left(buffer.cursors[i].position);
                                buffer.remove_at(*position);
                            }
                        },
                        Key::Return => for buffer in <&mut Buffer>::query().iter_mut(world) {
                            let positions = buffer.cursors.iter().map(|c| c.position).collect::<Vec<_>>();
        
                            for (i, position) in positions.iter().enumerate() {
                                buffer.insert_line(*position);
                                buffer.cursors[i].position.0 += 1;
                                buffer.cursors[i].position.1 = 0;
                            }
                        },
                        Key::Tab => for buffer in <&mut Buffer>::query().iter_mut(world) {
                            let positions = buffer.cursors.iter().map(|c| c.position).collect::<Vec<_>>();
                            
                            for (i, position) in positions.iter().enumerate() {
                                let new_position = buffer.insert_str_at("    ", *position);
                                buffer.cursors[i].position = new_position;
                            }

                        }
                        Key::Left => for buffer in <&mut Buffer>::query().iter_mut(world) {
                            for i in 0..buffer.cursors.len() {
                                buffer.cursors[i].position = buffer.move_left(buffer.cursors[i].position)
                            }
                        },
                        Key::Right => for buffer in <&mut Buffer>::query().iter_mut(world) {
                            for i in 0..buffer.cursors.len() {
                                buffer.cursors[i].position = buffer.move_right(buffer.cursors[i].position)
                            }
                        },
                        Key::Up => for buffer in <&mut Buffer>::query().iter_mut(world) {
                            for i in 0..buffer.cursors.len() {
                                buffer.cursors[i].position = buffer.move_up(buffer.cursors[i].position)
                            }
                        },
                        Key::Down => for buffer in <&mut Buffer>::query().iter_mut(world) {
                            for i in 0..buffer.cursors.len() {
                                buffer.cursors[i].position = buffer.move_down(buffer.cursors[i].position)
                            }
                        },
                        _ => {}
                    }
                }
            }
        },
        Event::MousePress(button, position, _) if matches!(button, MouseButton::Left) => {
            let mut buffers_and_positions = HashMap::new();

            for (buffer, view_ref) in <(&Buffer, &ViewRef)>::query().iter(world) {
                assert!(buffer.cursors.len() == 1);

                let view_entity = world.entry_ref(view_ref.0).unwrap();

                let view = view_entity.get_component::<View>().unwrap();
                let camera = view_entity.get_component::<Camera>().unwrap();

                if let Some(view_position) = view.to_view(position) {
                    let world_position = camera.view_to_world(view_position);
                    let buffer_position = buffer.buffer_position(world_position);

                    buffers_and_positions.insert(buffer.id, buffer_position);
                }
            }

            for buffer in <&mut Buffer>::query().iter_mut(world) {
                if let Some(buffer_position) = buffers_and_positions.get(&buffer.id) {
                    buffer.cursors[0].position = *buffer_position;
                } 
            }
        },*/
        Event::MouseScroll(scroll, position) => {
            for buffer_view in <&mut BufferView>::query().iter_mut(world) {
                if buffer_view.contains(position) {
                    buffer_view.scroll_vertically(scroll.y as f32);
                }
            }
        },
        _ => {}
    }
}