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
    pub fn matrix(&self) -> cgmath::Matrix4<f32> {
            // 1.
            let view = cgmath::Matrix4::look_at_rh(self.eye, self.target, self.up);
            // 2.
            let proj = cgmath::ortho(
                self.left, 
                self.left + self.width, 
                self.bottom, 
                self.bottom + self.height,
                0.0,
                100.0
            );
    
            // 3.
            return OPENGL_TO_WGPU_MATRIX * proj * view;
    }

    //this should be moved to some kind of camera controller
    // pub fn resize(&mut self, new_size: PhysicalSize) {

    // }
}