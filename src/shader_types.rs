use core::slice;
use std::{collections::HashMap, any::Any, num::NonZeroU64, mem, ptr};

use naga::{ImageDimension, ImageClass, ScalarKind};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub enum Vector<T> {
    Vec2([T; 2]),
    Vec3([T; 3]),
    Vec4([T; 4]),
}

impl<T: Default + Copy> Vector<T> {
    fn new(size: u32) -> Self {
        match size {
            2 => Self::Vec2([T::default(); 2]),
            3 => Self::Vec2([T::default(); 2]),
            4 => Self::Vec2([T::default(); 2]),
            _ => panic!("Creating weird vector size {size}")
        }
    }

    fn from(values: &[T]) -> Self {
        match values.len() {
            2 => Self::Vec2(values[0..2].try_into().unwrap()),
            3 => Self::Vec2(values[0..3].try_into().unwrap()),
            4 => Self::Vec2(values[0..4].try_into().unwrap()),
            size => panic!("Creating weird vector size {size}")
        }
    }
}

impl<T: Sized> Vector<T> {
    fn as_bytes(&self) -> &[u8] {
        match self {
            Vector::Vec2(v) => { 
                let bp = v.as_ptr() as *const _;
                unsafe { slice::from_raw_parts(bp, 2 * mem::size_of::<T>()) }
            },
            Vector::Vec3(v) => {
                let bp = v.as_ptr() as *const _;
                unsafe { slice::from_raw_parts(bp, 3 * mem::size_of::<T>()) }
            },
            Vector::Vec4(v) => {
                let bp = v.as_ptr() as *const _;
                unsafe { slice::from_raw_parts(bp, 4 * mem::size_of::<T>()) }
            },
        }
    }
}

#[derive(Debug, Clone)]
pub enum Matrix {
    Matrix4x4([[f32; 4]; 4])
}

impl Matrix {
    fn as_bytes(&self) -> &[u8] {
        match self {
            Matrix::Matrix4x4(v) => {
                let bp = v.as_ptr() as *const _;
                unsafe { slice::from_raw_parts(bp, 16 * mem::size_of::<f32>()) }
            },
        }
    }
}

impl Matrix {
    fn new(columns: u32, rows: u32) -> Option<Self> {
        if columns == 4 && rows == 4 {
            Some(Self::Matrix4x4([[0f32; 4]; 4]))
        } else {
            None
        }
    }

    fn from(matrix: [[f32; 4]; 4]) -> Self {
        Self::Matrix4x4(matrix)
    }
}

pub fn create_binding_type(naga_type: &naga::TypeInner) -> Option<wgpu::BindingType> {    
    let size = match &naga_type {
        naga::TypeInner::Scalar {width, .. } => 
            Some(NonZeroU64::new(*width as u64)),
        naga::TypeInner::Vector { size, width, .. } => 
            Some(NonZeroU64::new(*size as u64 * (*width as u64))),
        naga::TypeInner::Matrix { columns, rows, width } => 
            Some(NonZeroU64::new(*columns as u64 * *rows as u64 * (*width as u64))),
        naga::TypeInner::Struct { span, .. } =>
            Some(NonZeroU64::new(*span as u64)),
        _ => None
    };
    
    if let Some(size) = size {
        Some(wgpu::BindingType::Buffer { 
            ty: wgpu::BufferBindingType::Uniform, 
            has_dynamic_offset: false, 
            min_binding_size: size 
        })
    } else {
        let binding_type = match &naga_type {
            naga::TypeInner::Image { dim, arrayed, class } => 
                create_binding_type_for_image(*dim, *arrayed, *class)?,
            naga::TypeInner::Sampler { comparison } => 
                create_binding_type_for_sampler(*comparison),
            
            /*naga::TypeInner::Array { base, size, stride } => wgpu::BindingType::Buffer { 
                ty: wgpu::BufferBindingType::Uniform, 
                has_dynamic_offset: false, 
                min_binding_size: NonZeroU64::new(size as u64 * (stride as u64))
            },*/
    
            // naga::TypeInner::BindingArray { base, size } => todo!(),
            // naga::TypeInner::Atomic { kind, width } => todo!(),
            // naga::TypeInner::Pointer { base, space } => todo!(),
            // naga::TypeInner::ValuePointer { size, kind, width, space } => todo!(),
            _ => return None
        };

        Some(binding_type)
    }

}

pub fn create_uniform_storage(naga_type: &naga::TypeInner) -> Option<MaterialValue> {
    let value = match naga_type {
        naga::TypeInner::Scalar { kind, ..  } => match kind {
            naga::ScalarKind::Sint =>  MaterialValue::Int(0),
            naga::ScalarKind::Uint =>  MaterialValue::Uint(0),
            naga::ScalarKind::Float => MaterialValue::Float(0f32),
            naga::ScalarKind::Bool =>  MaterialValue::Bool(false),
        },
        naga::TypeInner::Vector { size, kind, .. } => match kind {
            naga::ScalarKind::Sint =>  MaterialValue::IntVector(Vector::<i32>::new(*size as u32)),
            naga::ScalarKind::Uint =>  MaterialValue::UintVector(Vector::<u32>::new(*size as u32)),
            naga::ScalarKind::Float => MaterialValue::FloatVector(Vector::<f32>::new(*size as u32)),
            naga::ScalarKind::Bool =>  MaterialValue::BoolVector(Vector::<bool>::new(*size as u32)),
        },
        naga::TypeInner::Matrix { columns, rows, .. } => 
            MaterialValue::Matrix(Matrix::new(*columns as u32, *rows as u32)?),
        
        // naga::TypeInner::Struct { members, .. } => todo!()),
        
        naga::TypeInner::Image { .. } => 
            MaterialValue::Texture(Texture::default()),
        naga::TypeInner::Sampler { .. } => 
            MaterialValue::Sampler(Sampler::default()),

        _ => return None
    };
    Some(value)
}

#[derive(Debug, Clone)]
pub enum MaterialValue {
    Float(f32),
    Int(i32),
    Uint(u32),
    Bool(bool),

    FloatVector(Vector<f32>),
    IntVector(Vector<i32>),
    UintVector(Vector<u32>),
    BoolVector(Vector<bool>),
    Matrix(Matrix),

    Texture(Texture),
    Sampler(Sampler),
    Struct(HashMap<String, MaterialValue>),
}

impl MaterialValue {
    pub fn as_bytes(&self) -> Option<&[u8]> {
        Some(match self {
            MaterialValue::Float(v) => {
                let bp = ptr::addr_of!(v) as *const u8;
                unsafe { slice::from_raw_parts(bp, mem::size_of::<f32>()) }
            },
            MaterialValue::Int(v) => {
                let bp = ptr::addr_of!(v) as *const u8;
                unsafe { slice::from_raw_parts(bp, mem::size_of::<f32>()) }
            },
            MaterialValue::Uint(v) => {
                let bp = ptr::addr_of!(v) as *const u8;
                unsafe { slice::from_raw_parts(bp, mem::size_of::<f32>()) }
            },
            MaterialValue::Bool(v) => if *v { &[1u8] } else { &[0u8] },
            MaterialValue::FloatVector(v) => v.as_bytes(),
            MaterialValue::IntVector(v) => v.as_bytes(),
            MaterialValue::UintVector(v) => v.as_bytes(),
            MaterialValue::BoolVector(v) => v.as_bytes(),
            MaterialValue::Matrix(v) => v.as_bytes(),
            MaterialValue::Struct(_) => return None,
            MaterialValue::Texture(_) => return None,
            MaterialValue::Sampler(_) => return None,
        })
    }

    pub fn get_mut<T: 'static>(&mut self) -> Option<&mut T> {
        let any: &mut dyn Any = match self {
            MaterialValue::Float(v) => v,
            MaterialValue::Int(v) => v,
            MaterialValue::Uint(v) => v,
            MaterialValue::Bool(v) => v,
            MaterialValue::FloatVector(v) => v,
            MaterialValue::IntVector(v) => v,
            MaterialValue::UintVector(v) => v,
            MaterialValue::BoolVector(v) => v,
            MaterialValue::Texture(v) => v,
            MaterialValue::Sampler(v) => v,
            MaterialValue::Struct(v) => v,
            MaterialValue::Matrix(v) => v,
        };

        any.downcast_mut::<T>()
    }

    pub fn get<T: 'static>(&self) -> Option<&T> {
        let any: &dyn Any = match self {
            MaterialValue::Float(v) => v,
            MaterialValue::Int(v) => v,
            MaterialValue::Uint(v) => v,
            MaterialValue::Bool(v) => v,
            MaterialValue::FloatVector(v) => v,
            MaterialValue::IntVector(v) => v,
            MaterialValue::UintVector(v) => v,
            MaterialValue::BoolVector(v) => v,
            MaterialValue::Texture(v) => v,
            MaterialValue::Sampler(v) => v,
            MaterialValue::Struct(v) => v,
            MaterialValue::Matrix(v) => v,
        };

        any.downcast_ref::<T>()
    }
}

#[derive(PartialEq, Eq, Debug, Clone, Default)]
pub struct Texture {
    pub uuid: Option<Uuid>,
}

impl Texture {
    pub fn new(uuid: Uuid) -> Self {
        Self {
            uuid: Some(uuid)
        }
    }
}

fn create_binding_type_for_image(dim: ImageDimension, arrayed: bool, class: ImageClass) -> Option<wgpu::BindingType> {
    let dim = match dim {
        ImageDimension::D1 => if arrayed { return None } else { wgpu::TextureViewDimension::D1 },
        ImageDimension::D2 => if arrayed { wgpu::TextureViewDimension::D2Array } else { wgpu::TextureViewDimension::D2 },
        ImageDimension::D3 => if arrayed { return None } else { wgpu::TextureViewDimension::D3 },
        ImageDimension::Cube => if arrayed { wgpu::TextureViewDimension::CubeArray } else { wgpu::TextureViewDimension::Cube }
    };

    Some(match class {
        ImageClass::Sampled { kind, multi } => {
            wgpu::BindingType::Texture {
                sample_type: match kind {
                    ScalarKind::Sint => wgpu::TextureSampleType::Sint,
                    ScalarKind::Uint => wgpu::TextureSampleType::Uint,
                    ScalarKind::Float => wgpu::TextureSampleType::Float { filterable: true },
                    _ => return None
                },
                view_dimension: dim,
                multisampled: multi,
            }
        },
        ImageClass::Depth { multi } => wgpu::BindingType::Texture { 
            sample_type: wgpu::TextureSampleType::Depth, 
            view_dimension: dim, 
            multisampled: multi
        },
        ImageClass::Storage { format, access } => wgpu::BindingType::StorageTexture { 
            access: match access.bits() {
                1 => wgpu::StorageTextureAccess::ReadOnly,
                2 => wgpu::StorageTextureAccess::WriteOnly,
                3 => wgpu::StorageTextureAccess::ReadWrite,
                _ => return None
            },
            format: match format {
                naga::StorageFormat::R8Unorm => wgpu::TextureFormat::R8Unorm,
                naga::StorageFormat::R8Snorm => wgpu::TextureFormat::R8Uint,
                naga::StorageFormat::R8Uint => wgpu::TextureFormat::R8Uint,
                naga::StorageFormat::R8Sint => wgpu::TextureFormat::R8Sint,
                naga::StorageFormat::R16Uint => wgpu::TextureFormat::R16Uint,
                naga::StorageFormat::R16Sint => wgpu::TextureFormat::R16Sint,
                naga::StorageFormat::R16Float => wgpu::TextureFormat::R16Float,
                naga::StorageFormat::Rg8Unorm => wgpu::TextureFormat::Rg8Unorm,
                naga::StorageFormat::Rg8Snorm => wgpu::TextureFormat::Rg8Snorm,
                naga::StorageFormat::Rg8Uint => wgpu::TextureFormat::Rg8Uint,
                naga::StorageFormat::Rg8Sint => wgpu::TextureFormat::Rg8Sint,
                naga::StorageFormat::R32Uint => wgpu::TextureFormat::R32Uint,
                naga::StorageFormat::R32Sint => wgpu::TextureFormat::R32Sint,
                naga::StorageFormat::R32Float => wgpu::TextureFormat::R32Float,
                naga::StorageFormat::Rg16Uint => wgpu::TextureFormat::Rg16Uint,
                naga::StorageFormat::Rg16Sint => wgpu::TextureFormat::Rg16Sint,
                naga::StorageFormat::Rg16Float => wgpu::TextureFormat::Rg16Float,
                naga::StorageFormat::Rgba8Unorm => wgpu::TextureFormat::Rgba8Unorm,
                naga::StorageFormat::Rgba8Snorm => wgpu::TextureFormat::Rgba8Snorm,
                naga::StorageFormat::Rgba8Uint => wgpu::TextureFormat::Rgba8Uint,
                naga::StorageFormat::Rgba8Sint => wgpu::TextureFormat::Rgba8Sint,
                naga::StorageFormat::Rgb10a2Unorm => wgpu::TextureFormat::Rgb10a2Unorm,
                naga::StorageFormat::Rg11b10Float => wgpu::TextureFormat::Rg11b10Float,
                naga::StorageFormat::Rg32Uint => wgpu::TextureFormat::Rg32Uint,
                naga::StorageFormat::Rg32Sint => wgpu::TextureFormat::Rg32Sint,
                naga::StorageFormat::Rg32Float => wgpu::TextureFormat::Rg32Float,
                naga::StorageFormat::Rgba16Uint => wgpu::TextureFormat::Rgba16Uint,
                naga::StorageFormat::Rgba16Sint => wgpu::TextureFormat::Rgba16Sint,
                naga::StorageFormat::Rgba16Float => wgpu::TextureFormat::Rgba16Float,
                naga::StorageFormat::Rgba32Uint => wgpu::TextureFormat::Rgba32Uint,
                naga::StorageFormat::Rgba32Sint => wgpu::TextureFormat::Rgba32Sint,
                naga::StorageFormat::Rgba32Float => wgpu::TextureFormat::Rgba32Float,
                naga::StorageFormat::R16Unorm => wgpu::TextureFormat::R16Unorm,
                naga::StorageFormat::R16Snorm => wgpu::TextureFormat::R16Snorm,
                naga::StorageFormat::Rg16Unorm => wgpu::TextureFormat::Rg16Unorm,
                naga::StorageFormat::Rg16Snorm => wgpu::TextureFormat::Rg16Snorm,
                naga::StorageFormat::Rgba16Unorm => wgpu::TextureFormat::Rgba16Unorm,
                naga::StorageFormat::Rgba16Snorm => wgpu::TextureFormat::Rgba16Snorm,
            }, 
            view_dimension: dim
        }
    })
}

#[derive(Debug, Clone, Default)]
pub struct Sampler {
    pub uuid: Option<Uuid>
}

impl Sampler {
    pub fn new(uuid: Uuid) -> Self {
        Self {
            uuid: Some(uuid)
        }
    }
}

fn create_binding_type_for_sampler(comparison: bool) -> wgpu::BindingType {
    if comparison {
        wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering)
    } else {
        wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering)
    }
}