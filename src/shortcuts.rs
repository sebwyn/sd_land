use legion::{World, Entity, IntoQuery, query};

use crate::{system::{Event, Key}, graphics::Visible, file_searcher::FileSearcher, app::EnttRef};

pub fn trigger_shortcuts(world: &mut World, event: &Event) {
    if let Event::KeyPress(key, modifiers) = event {
        //command + p: toggles the file searcher
        if modifiers.logo() && matches!(key, Key::Char('p', ..)) {
            println!("Opening the find window!");

            let mut query = <(&EnttRef, &FileSearcher)>::query();
                if let Some((EnttRef(entity), file_searcher)) = query.iter(world).next() {
                    let entry = world.entry(*entity);
                    if entry.is_none() {
                        return;
                    }
                    let mut entry = entry.unwrap();

                    //toggle the visibility
                    if entry.get_component::<Visible>().into_iter().next().is_some() {
                        entry.remove_component::<Visible>();
                    } else {
                        entry.add_component(Visible);
                    }
            }

        }
    }
}
