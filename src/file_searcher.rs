use legion::{World, Entity};
use simple_error::SimpleError;

use crate::{camera::Camera, view::{View, ViewRef}, system::Event, ui_box::UiBoxFactory, text::Font};

pub struct FileSearcher;

pub fn emplace_find_menu(world: &mut World, font: Font, ui_box_factory: &UiBoxFactory) -> Result<Entity, SimpleError> {
    //create
    let width = 800;
    let height = 600;

    let camera = Camera::new(width, height); //the default size for this thing
    let view = View::new(1200, 1200 + width, 100 + height, 100, 0.0, 1.0); //center it in the default window frame

    let view_entity = world.push((FileSearcher, camera, view));

    //create the Ui Box for this thing
    let vertices = ui_box_factory.create("#424B54", (0f32, 0f32), (width as f32, height as f32), 0.5)?;
    world.push((vertices, ui_box_factory.material(), ViewRef(view_entity)));

    Ok(view_entity)
}

pub fn file_searcher_on_event(world: &mut World, event: &Event) {

}