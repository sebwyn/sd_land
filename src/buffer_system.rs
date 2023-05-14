use std::collections::HashMap;

use legion::{Entity, World, EntityStore, IntoQuery};
use winit::event::MouseButton;

use crate::{system::{Event, Key}, renderer::{view::{ViewRef, View}, camera::Camera, primitive::{Vertex, RectangleBuilder}}, buffer::Buffer};

#[derive(Clone, Copy)]
pub struct Cursor {
    pub entity: Entity,
    pub position: (usize, usize),
}

pub fn buffer_on_event(world: &mut World, event: &Event) { 
    match event {
        Event::PrepareRender => {
            let mut camera_query = <(&Buffer, &ViewRef)>::query();

            let mut cameras = HashMap::new();

            for (_, view) in camera_query.iter(world) {
                
                cameras.entry(view.0).or_insert_with(|| {
                    let camera_entity = world.entry_ref(view.0).expect("Expected buffer to be in a view");
                    let camera = camera_entity.get_component::<Camera>()
                        .expect("Expected View to have an associated camera!");

                    camera.clone()
                });
            }

            let mut buffer_query = <(&Buffer, &mut Vec<Vertex>, &ViewRef)>::query();
            
            let mut cursors: Vec<(Cursor, Vec<Vertex>)> = Vec::new();

            for (buffer, vertices, view) in buffer_query.iter_mut(world) {
                let camera = cameras.get(&view.0).expect("No camera found for view!");
                
                let view_top = camera.view_top();
                let view_bottom = camera.view_bottom();

                let new_vertices = buffer.render(view_top, view_bottom);

                *vertices = new_vertices;

                for cursor in &buffer.cursors {
                    //generate a position and a rectangle for the cursor
                    let (world_x, mut world_y) = buffer.world_position(cursor.position);
                    world_y += buffer.font.smallest_ymin(buffer.font_scale);
                    // let height = buffer.font.font_height(buffer.font_scale);

                    let vertices = RectangleBuilder::default()
                        .position(world_x, world_y)
                        .size(3f32, buffer.line_height)
                        .depth(0.4)
                        .build();
                    
                    cursors.push((*cursor, vertices));
                }
            }

            //draw the cursors
            for (cursor, new_vertices) in cursors {
                let mut cursor_entity = world.entry(cursor.entity).unwrap();
                let vertices = cursor_entity.get_component_mut::<Vec<Vertex>>().unwrap();
                *vertices = new_vertices;
            }
        },
        Event::KeyPress(key, modifiers) => {
            if modifiers.logo() && !modifiers.shift() && !modifiers.alt() {
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
            }
            if !modifiers.logo() && !modifiers.alt() {
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
        },
        Event::MouseScroll(scroll, position) => {
            let mut query = <(&Buffer, &ViewRef)>::query();
            
            //sort the elements by depth so we find the one on top

            let view_entities = 
                query.iter(world).map(|(_, view)| view.0).collect::<Vec<_>>();

            for entity in view_entities {
                let mut view_entry = match world.entry(entity) {
                    Some(entry) => entry,
                    None => continue,
                };

                let view = match view_entry.get_component_mut::<View>() {
                    Ok(view) => view,
                    Err(_) => continue,
                };

                if view.contains_point(position) {
                    let camera = match view_entry.get_component_mut::<Camera>() {
                        Ok(camera) => camera,
                        Err(_) => continue,
                    };

                    //scroll the camera
                    camera.eye.y += scroll.y as f32;
                    camera.target.y = camera.eye.y;

                    break;
                }
            }
        },
        _ => {}
    }
}