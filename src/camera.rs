use cgmath::{Point3, Vector3};
use winit::dpi::PhysicalPosition;

use crate::graphics::Vertex;

#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.0,
    0.0, 0.0, 0.5, 1.0,
);

pub struct Camera {
    pub eye: cgmath::Point3<f32>,
    pub target: cgmath::Point3<f32>,
    up: cgmath::Vector3<f32>, 
    bottom: f32,
    left: f32,
    width: f32,
    height: f32
}

impl Camera {
    pub fn new(width: u32, height: u32) -> Self {
        let bottom = 0.0;
        let left = 0.0;
        let width = width as f32;
        let height = height as f32;

        let eye = Point3::<f32> { x: 0.0, y: 0.0, z: 1.0 };
        let target = Point3::<f32> { x: 0.0, y: 0.0, z: 0.0 };
        let up = Vector3::<f32> { x: 0.0, y: 1.0, z: 0.0 };

        Self {
            eye,
            target,
            up,
            bottom,
            left,
            width,
            height
        }
    }

    pub fn view_bottom(&self) -> f32 {
        self.eye.y - 50f32
    }

    pub fn view_top(&self) -> f32 {
        (self.eye.y + self.height) + 50f32
    }

    pub fn matrix(&self) -> cgmath::Matrix4<f32> {
            let view = cgmath::Matrix4::look_at_rh(self.eye, self.target, self.up);
            // let view = cgmath::Matrix4::identity();
            let proj = cgmath::ortho(
                self.left, 
                self.left + self.width, 
                self.bottom, 
                self.bottom + self.height,
                0.0,
                1.0
            );
    
            OPENGL_TO_WGPU_MATRIX * proj * view
    }

    pub fn is_visible(&self, vertices: &[Vertex]) -> bool {
        let top = (self.eye.y + self.height) + 30f32;
        let bottom = self.eye.y - 30f32;
        
        //just do a super simple check
        if bottom < vertices[0].position()[1] && vertices[0].position()[1] < top {
            true
        } else {
            false
        }
    }

    pub fn contains_point(&self, point: &PhysicalPosition<f64>) -> bool {    
        let top = self.eye.y + self.height;
        let bottom = self.eye.y; 
        let right = self.eye.x + self.width;
        let left = self.eye.x; 

        if left < point.x as f32 && right > point.x as f32 {
            if bottom < point.y as f32 && top > point.y as f32 {
                return true;
            }
        }
        false
    }

    pub fn view_to_world(&self, point: &PhysicalPosition<f64>) -> (f32, f32) {
        println!("{:?}", self.eye);
        (self.eye.x + point.x as f32, (self.eye.y + self.height) - point.y as f32)
    }
}

// pub fn camera_on_event(world: &mut World, event: &Event) {
//     match event {
//         Event::Resize(new_size) => {
//             let mut camera_query = <&mut Camera>::query();
    
//             // for camera in camera_query.iter_mut(world) {
//             //     camera.width = new_size.width as f32;
//             //     camera.height = new_size.height as f32;
//             // }
//         }
//         Event::MouseScroll(PhysicalPosition::<f64> { y, .. }, ..) => {
//             let mut camera_query = <&mut Camera>::query()
//                 .filter(!component::<FileSearcher>());
    
//             for camera in camera_query.iter_mut(world) {
//                 camera.eye.y += *y as f32;
//                 camera.target.y = camera.eye.y;

//                 // camera.eye.x -= *x as f32;
//                 // camera.target.x = camera.eye.x;

//             }
//         },
//         _ => {}
//     }
// }