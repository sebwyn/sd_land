#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    position: [f32; 3],
    color: [f32; 3],
    tex_coords: [f32; 2],
}

impl Vertex {
    const ATTRIBS: [wgpu::VertexAttribute; 3] =
        wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x2];
}

impl crate::pipeline::Vertex for Vertex {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        use std::mem;

        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

pub struct RectangleBuilder {
    x: f32, 
    y: f32, 
    width: f32, 
    height: f32, 
    depth: f32, 
    color: [f32; 3],
    tex_coords: [[f32; 2]; 4]
}

impl Default for RectangleBuilder {
    fn default() -> Self {
        Self { x: 0.0, y: 0.0, width: 1.0, height: 1.0, depth: 0.0, color: [1.0, 1.0, 1.0], tex_coords: [[0.0, 1.0], [0.0, 0.0], [1.0, 1.0], [1.0, 0.0]] }
    }
}

impl RectangleBuilder {
    pub fn position(mut self, x: f32, y: f32) -> Self {
        self.x = x; self.y = y; self
    }

    pub fn size(mut self, width: f32, height: f32) -> Self {
        self.width = width; self.height = height; self
    }

    pub fn depth(mut self, depth: f32) -> Self {
        self.depth = depth; self
    }

    pub fn color(mut self, color: [f32; 3]) -> Self {
        self.color = color; self
    }

    pub fn tex_coords(mut self, tex_coords: [[f32; 2]; 4]) -> Self {
        self.tex_coords = tex_coords; self
    }

    pub fn build(self) -> Rectangle {
        
        Rectangle {
            vertices: [
                Vertex { position: [self.x,            self.y,             self.depth], color: self.color, tex_coords: self.tex_coords[0] }, 
                Vertex { position: [self.x,            self.y+self.height, self.depth], color: self.color, tex_coords: self.tex_coords[1] }, 
                Vertex { position: [self.x+self.width, self.y,             self.depth], color: self.color, tex_coords: self.tex_coords[2] }, 
                Vertex { position: [self.x+self.width, self.y+self.height, self.depth], color: self.color, tex_coords: self.tex_coords[3] }, 
            ]
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rectangle {
    pub vertices: [Vertex; 4]
}

impl Rectangle {
    pub const INDICES: [u32; 6] = [0, 2, 1, 1, 2, 3];
}