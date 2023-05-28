use cgmath::Point3;
use legion::{component, system};
use legion::systems::Builder;
use winit::event::MouseButton;
use crate::event::{Event, MouseDrag};
use crate::renderer::camera::Camera;
use crate::sprite_renderer::ActiveSceneCamera;


pub fn add_scene_camera_controller(schedule: &mut Builder) {
    schedule.add_system(control_camera_system(SceneCameraController::default()));
}


#[derive(Default)]
struct SceneCameraController {
    starting_position: (f32, f32)
}

#[system(for_each)]
#[write_component(Camera)]
#[filter(component::<ActiveSceneCamera>())]
fn control_camera(#[state] controller: &mut SceneCameraController, camera: &mut Camera, #[resource] events: &Vec<Event>, #[resource] screen_size: &(f32, f32)) {
    for event in events {
        match event {
            Event::MousePress(..) => {
                controller.starting_position = (camera.eye.x, camera.eye.y);
            },
            Event::MouseDrag(MouseDrag{
                 start,
                 current_position,
                 button: MouseButton::Left,
                 ..
            }) => {
                let translation = (
                    ((current_position.x - start.x) as f32 / screen_size.0) * camera.width,
                    ((current_position.y - start.y) as f32 / screen_size.1) * camera.height,
                );

                camera.eye = Point3 {
                    x: controller.starting_position.0 - translation.0,
                    y: controller.starting_position.1 + translation.1,
                    z: camera.eye.z,
                };
                camera.target.x = camera.eye.x;
                camera.target.y = camera.eye.y;
            },
            //TODO: the y position doesn't look like it remains constant when zooming (fix this!)
            Event::MouseScroll(scroll, position, ..) => {
                let zoom_multiplier = (1.0 + 0.01 * scroll.y.signum()) as f32;

                let normalized_point = (position.x as f32 / screen_size.0, 1.0 - position.y as f32 / screen_size.1);

                let starting_point = (camera.width * normalized_point.0, camera.height * normalized_point.1);
                camera.width *= zoom_multiplier;
                camera.height *= zoom_multiplier;
                let ending_point = (camera.width * normalized_point.0, camera.height * normalized_point.1);

                let translation = (starting_point.0 - ending_point.0, starting_point.1 - ending_point.1);
                camera.eye = Point3 {
                    x: camera.eye.x + translation.0,
                    y: camera.eye.y + translation.1,
                    z: camera.eye.z,
                };
                camera.target.x = camera.eye.x;
                camera.target.y = camera.eye.y;

            },
            _ => {}
        }
    }
}