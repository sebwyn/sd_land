pub struct Visible;

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, bytemuck::Pod, bytemuck::Zeroable, Default)]
pub struct Rectangle {
    position: [f32; 2],
    dimensions: [f32; 2],

    color: [f32; 4],

    tex_position: [f32; 2],
    tex_dimensions: [f32; 2],

    corner_radius: f32,

    border_width: f32,
    border_color: [f32; 3],

    depth: f32
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, bytemuck::Pod, bytemuck::Zeroable, Default)]
pub struct RectangleVertex {
    position: [f32; 2],
    tex_coords: [f32; 2]
}

impl RectangleVertex {
    const ATTRIBS: [wgpu::VertexAttribute; 2] =
        wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2];
}

impl crate::renderer::pipeline::Vertex for RectangleVertex {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        use std::mem;

        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

impl Rectangle {
    pub const INDICES: [u32; 6] = [0, 2, 1, 1, 2, 3];
    
    pub const VERTICES: [RectangleVertex; 4] = [
        RectangleVertex { position: [0.0, 0.0], tex_coords: [0.0, 1.0] },
        RectangleVertex { position: [0.0, 1.0], tex_coords: [0.0, 0.0] },
        RectangleVertex { position: [1.0, 0.0], tex_coords: [1.0, 1.0] },
        RectangleVertex { position: [1.0, 1.0], tex_coords: [1.0, 0.0] },
    ];

    // pub const TEX_COORDS: [[f32; 2]; 4] = [
    //     [0.0, 1.0], 
    //     [0.0, 0.0], 
    //     [1.0, 1.0], 
    //     [1.0, 0.0]
    // ];

    pub fn position(mut self, position: [f32; 2]) -> Self {
        self.position = position; self
    }

    pub fn dimensions(mut self, dimensions: [f32; 2]) -> Self {
        self.dimensions = dimensions; self
    }

    pub fn tex_position(mut self, tex_position: [f32; 2]) -> Self {
        self.tex_position = tex_position; self
    }

    pub fn tex_dimensions(mut self, tex_dimensions: [f32; 2]) -> Self {
        self.tex_dimensions = tex_dimensions; self
    }

    pub fn color(mut self, color: [f32; 3]) -> Self {
        self.color = [color[0], color[1], color[2], self.color[3]]; self
    }

    pub fn opacity(mut self, opacity: f32) -> Self {
        self.color[3] = opacity; self
    }

    pub fn depth(mut self, depth: f32) -> Self {
        self.depth = depth; self
    }

    pub fn corner_radius(mut self, corner_radius: f32) -> Self {
        self.corner_radius = corner_radius; self
    }

    pub fn border_width(mut self, border_width: f32) -> Self {
        self.border_width = border_width; self
    }

    pub fn border_color(mut self, border_color: [f32; 3]) -> Self {
        self.border_color = border_color; self
    }
    
}

impl crate::renderer::pipeline::Vertex for Rectangle {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Rectangle>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 5,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                    shader_location: 6,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 7,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 8]>() as wgpu::BufferAddress,
                    shader_location: 8,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 10]>() as wgpu::BufferAddress,
                    shader_location: 9,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 12]>() as wgpu::BufferAddress,
                    shader_location: 10,
                    format: wgpu::VertexFormat::Float32,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 13]>() as wgpu::BufferAddress,
                    shader_location: 11,
                    format: wgpu::VertexFormat::Float32,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 14]>() as wgpu::BufferAddress,
                    shader_location: 12,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 17]>() as wgpu::BufferAddress,
                    shader_location: 13,
                    format: wgpu::VertexFormat::Float32,
                },
            ],
        }
    }
}   



#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    position: [f32; 3],
    color: [f32; 4],
    tex_coords: [f32; 2],
}

impl Vertex {
    const ATTRIBS: [wgpu::VertexAttribute; 3] =
        wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x4, 2 => Float32x2];

    pub fn position(&self) -> &[f32; 3] { &self.position }
}

impl super::pipeline::Vertex for Vertex {
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
    opacity: f32,
    tex_coords: [[f32; 2]; 4]
}

impl Default for RectangleBuilder {
    fn default() -> Self {
        Self { x: 0.0, y: 0.0, width: 1.0, height: 1.0, depth: 0.0, color: [1.0, 1.0, 1.0], tex_coords: [[0.0, 1.0], [0.0, 0.0], [1.0, 1.0], [1.0, 0.0]], opacity: 1.0 }
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

    pub fn opacity(mut self, opacity: f32) -> Self {
        self.opacity = opacity; self
    }

    pub fn build(self) -> Vec<Vertex> {
        vec![
            //bottom left
            Vertex { position: [self.x,            self.y,             self.depth], color: [self.color[0], self.color[1], self.color[2], self.opacity], tex_coords: self.tex_coords[0] }, 
            //top left
            Vertex { position: [self.x,            self.y+self.height, self.depth], color: [self.color[0], self.color[1], self.color[2], self.opacity], tex_coords: self.tex_coords[1] }, 
            //bottom right
            Vertex { position: [self.x+self.width, self.y,             self.depth], color: [self.color[0], self.color[1], self.color[2], self.opacity], tex_coords: self.tex_coords[2] }, 
            //top right
            Vertex { position: [self.x+self.width, self.y+self.height, self.depth], color: [self.color[0], self.color[1], self.color[2], self.opacity], tex_coords: self.tex_coords[3] }, 
        ]
    }
}