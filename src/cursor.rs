use legion::{World, IntoQuery, Entity, EntityStore};
use winit::event::MouseButton;

use crate::{system::{Event, Key}, buffer::Buffer, view::View, camera::Camera};

pub struct Cursor {
    buffer: Entity,
    camera: Entity,
    position: (usize, (usize, usize)), //row column position in the buffer
}

impl Cursor {
    pub fn new(buffer: Entity, camera: Entity) -> Self {
        Self {
            buffer,
            camera,
            position: (0, (0, 0))
        }
    }
}

pub fn cursor_on_event(world: &mut World, event: &Event) {
    
    match event {
        Event::MousePress(button, position) if matches!(button, MouseButton::Left) => {
            let mut cursor_query = <&Cursor>::query();
            
            let mut buffer_positions = Vec::new();
            for cursor in cursor_query.iter(world) {
                
                //insert a character at this position in the buffer
                let camera_view_entity = match world.entry_ref(cursor.camera) {
                    Ok(camera) => camera,
                    Err(_) => continue,
                };
                let camera = match camera_view_entity.get_component::<Camera>() {
                    Ok(camera) => camera,
                    Err(_) => continue,
                };
                let view = match camera_view_entity.get_component::<View>() {
                    Ok(camera) => camera,
                    Err(_) => continue,
                };

                let world_position;
                if view.contains_point(position) {
                    world_position = camera.view_to_world(position)
                } else {
                    continue
                }

                let buffer_entry = match world.entry_ref(cursor.buffer.clone()) {
                    Ok(buffer) => buffer,
                    Err(_) => continue, //stranded cursor
                };
                let buffer = match buffer_entry.get_component::<Buffer>() {
                    Ok(buffer) => buffer,
                    Err(_) => continue,
                };


                println!("{:?}", world_position);
                println!("{:?}", position);
                buffer_positions.push(buffer.buffer_position(world_position));
            }

            let mut cursor_query = <&mut Cursor>::query();

            for (i, cursor) in cursor_query.iter_mut(world).enumerate() {
                cursor.position = buffer_positions[i];
            }

        },
        Event::KeyPress(key, modifiers) => {

            let character = match key {
                Key::Char(_, uppercase) if modifiers.shift() && uppercase.is_some() => Some(uppercase.unwrap()),
                Key::Char(lowercase, _) if !modifiers.shift() => Some(*lowercase),
                Key::Tab => Some('\t'),
                Key::Return => Some('\n'),
                _ => None
            };
            
            let mut cursor_query = <&Cursor>::query();
                let buffer_and_position = cursor_query.iter(world).map(|c| (c.buffer, c.position)).collect::<Vec<_>>();
            
                for (buffer_entity, position) in buffer_and_position {
                //insert a character at this position in the buffer
                let mut buffer_entry = match world.entry(buffer_entity) {
                    Some(buffer) => buffer,
                    None => continue, //stranded cursor
                };

                let buffer = match buffer_entry.get_component_mut::<Buffer>() {
                    Ok(buffer) => buffer,
                    Err(_) => continue,
                };
                
                if let Some(character) = character {
                    buffer.insert_at(character, position);
                } else if matches!(key, Key::Backspace) {
                    buffer.remove_at(position)
                }
            }

            let mut cursor_query = <&mut Cursor>::query();
            for cursor in cursor_query.iter_mut(world) {
                
                if let Some(character) = character {
                    cursor.position.0 += 1;
                    cursor.position.1.1 += 1;
                } else if matches!(key, Key::Backspace) {
                    cursor.position.0 -= 1;
                    cursor.position.1.1 -= 1;
                }
            }
        },
        _ => {}
    }
}

