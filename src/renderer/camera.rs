use cgmath::{Point3, Vector3};
use winit::dpi::PhysicalPosition;

use super::primitive::Vertex;

#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.0,
    0.0, 0.0, 0.5, 1.0,
);

#[derive(Clone)]
pub struct Camera {
    pub eye: Point3<f32>,
    pub target: Point3<f32>,
    up: Vector3<f32>,
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

    pub fn contains_point(&self, point: &PhysicalPosition<f64>) -> bool {    
        let top = self.eye.y + self.height;
        let bottom = self.eye.y; 
        let right = self.eye.x + self.width;
        let left = self.eye.x; 

        left < point.x as f32 && right > point.x as f32 && 
        bottom < point.y as f32 && top > point.y as f32
    }

    pub fn view_to_world(&self, point: (f32, f32)) -> (f32, f32) {
        (self.eye.x + point.0, (self.eye.y + self.height) - point.1)
    }
}