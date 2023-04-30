use cgmath::{Point3, Vector3};
use winit::dpi::PhysicalSize;

#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.0,
    0.0, 0.0, 0.5, 1.0,
);

pub struct Camera {
    eye: cgmath::Point3<f32>,
    target: cgmath::Point3<f32>,
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

    //this should be moved to some kind of camera controller
    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        self.width = new_size.width as f32;
        self.height = new_size.height as f32;
        self.eye = Point3::<f32> { x: self.width / 2.0, y: self.height / 2.0, z: 100.0 };
        self.target = Point3::<f32> { x: self.width / 2.0, y: self.height / 2.0, z: 0.0 };
    }
}