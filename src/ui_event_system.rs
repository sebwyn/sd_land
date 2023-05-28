use legion::{World, IntoQuery, Entity};

use crate::{layout::Transform, text_renderer::TextBox};
use crate::event::{Event, Key};

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