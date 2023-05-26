use legion::{World, IntoQuery, Entity};

use crate::{layout::Transform, system::{Event, Systems, Key}, text_renderer::TextBox};

#[derive(Default)]
pub struct UserEventListener {
    pub on_key_event: Option<fn(Event, Entity, &mut World)>,
    pub on_mouse_event: Option<fn(Event, Entity, &mut World)>
}

pub fn text_box_on_key_event(event: Event, entity: Entity, world: &mut World) {
    let key = match event {
        Event::KeyPress(Key::Char(_, Some(uppercase)), modifiers) if modifiers.shift() => uppercase,
        Event::KeyPress(Key::Char(lowercase, _), _) => lowercase,
        _ => return
    };
    
    if let Some(mut entry) = world.entry(entity) {
        if let Ok(text_box) = entry.get_component_mut::<TextBox>() {
            text_box.text += &String::from(key);
        }
    }
    
}

pub fn ui_on_event(event: &Event, world: &mut World, systems: &Systems) {
    match event {
        //keypresses maybe operate off of some kind of 'focus'
        Event::KeyPress(..) | Event::KeyRelease(..) => {
            //but for now, have every element react to every keypress
            //do some kind of accelerated bounds checking against a ton of possible transforms
            let key_listeners = <(Entity, &UserEventListener)>::query().iter(world)
                .filter_map(|(entity, listener)| 
                    listener.on_key_event.map(|key_listener| (*entity, key_listener))
                )
                .collect::<Vec<_>>();

            for (entity, listener) in key_listeners {
                listener(*event, entity, world)
            }
        },
        //mouse events, we need to filter the elements to see if the click was on the element
        Event::MouseScroll(_, position, _) | Event::MousePress(_, position, _) | 
        Event::MouseMoved(_, position, _) | Event::MouseRelease(_, position, _) | 
        Event::MouseClick(_, position, _) => {
            let screen_size = systems.screen_size();
            let world_coord = (position.x as f32, screen_size.1 - position.y as f32);
            //do some kind of accelerated bounds checking against a ton of possible transforms
            //TODO: make it accelerated
            let mouse_listeners = <(Entity, &UserEventListener, &Transform)>::query().iter(world)
                .filter_map(|(entity, listener, transform)| {
                    listener.on_mouse_event.map(|mouse_listener|
                    if transform.contains_point(world_coord) {
                        Some((*entity, mouse_listener))
                    } else {
                        None
                    })?
                })
                .collect::<Vec<_>>();

            for (entity, listener) in mouse_listeners {
                listener(*event, entity, world);
            }
        }
        _ => {}
    }
}