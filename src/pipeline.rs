use std::collections::HashMap;

use naga::{ResourceBinding, Module, Handle, GlobalVariable, Expression};
use simple_error::SimpleError;
use wgpu::VertexBufferLayout;

use crate::{shader_types::{create_binding_type, create_uniform_storage}, material::Material};

#[derive(Clone)]
pub struct Uniform {
    pub binding: ResourceBinding,
    pub binding_type: wgpu::BindingType,
    pub visibility: wgpu::ShaderStages,
    pub naga_type: naga::TypeInner,
}

impl<'a> Uniform {
    fn new(binding: ResourceBinding, binding_type: wgpu::BindingType, visibility: wgpu::ShaderStages, naga_type: naga::TypeInner) -> Self {
        Self {
            binding,
            binding_type,
            visibility,
            naga_type
        }
    }
}

pub trait Vertex {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a>;
}

#[derive(Clone)]
pub struct Pipeline {
    shader_source: String,
    uniforms: HashMap<String, Uniform>,
    vs_entry_point: String,
    fs_entry_point: String,
    vertex_buffer_layout: Option<wgpu::VertexBufferLayout<'static>>
}

impl Pipeline {
    pub fn load<T: Vertex>(shader: &str) -> Result<Self, SimpleError> {
        let shader_cource = String::from(shader);

        let shader_module = naga::front::wgsl::parse_str(&shader_cource).expect("Failed to load shader!");

        let vs_entry_point = shader_module.entry_points.iter()
            .find(|entry_point| entry_point.stage == naga::ShaderStage::Vertex)
            .ok_or(SimpleError::new("Could not find vertex program defined in shader!"))?
            .function.name
            .as_ref()
            .ok_or(SimpleError::new("Could not find name for vertex function!"))?
            .clone();

        let fs_entry_point = shader_module.entry_points.iter()
            .find(|entry_point| entry_point.stage == naga::ShaderStage::Fragment)
            .expect("Could not find vertex program defined in shader!")
            .function.name
            .as_ref()
            .expect("Could not find name for vertex function!")
            .clone();

        let mut uniforms = Self::parse_shader_uniforms(&shader_module)?;
        
        let vertex_buffer_layout = Some(T::desc());

        Self::correct_filterable_samplers(&mut uniforms);

        Ok(Self {
            shader_source: shader.to_string(),
            uniforms,
            vs_entry_point,
            fs_entry_point,
            vertex_buffer_layout
        })
    }

    pub fn bind_groups(&self) -> Vec<Vec<&Uniform>> {
        let mut groups: HashMap<u32, Vec<&Uniform>> = HashMap::new();
        
        for (_, uniform) in self.uniforms.iter() {
            groups.entry(uniform.binding.group)
                .and_modify(|group| group.push(uniform))
                .or_insert(vec![uniform]);
        }

        //need to correct unfilterable samplers

        groups.into_iter().map(|(_, uniforms)| uniforms).collect::<Vec<_>>()
    }

    pub fn new_material(&self) -> Material {
        let uniforms = self.uniforms.iter()
            .map(|(name, uniform)| {
                let binding = (uniform.binding.group, uniform.binding.binding);
                let uniform_storage = create_uniform_storage(&uniform.naga_type)
                    .expect(&format!("Failed to create storage for uniform: {}", name)); 

                (name.clone(), binding, uniform_storage)
            })
            .collect::<Vec<_>>();

        Material::new(uniforms)
    }

    pub fn shader(&self) -> &str { &self.shader_source }
    pub fn vs_entry_point(&self) -> &str { &self.vs_entry_point }
    pub fn fs_entry_point(&self) -> &str { &self.fs_entry_point }
    pub fn buffer_layouts(&self) -> &[VertexBufferLayout] { self.vertex_buffer_layout.as_slice() }

    fn parse_shader_uniforms(shader_module: &Module) -> Result<HashMap<String, Uniform>, SimpleError> {
        let mut uniforms: HashMap<String, Uniform> = HashMap::new();
        
        let naga_types = &shader_module.types;

        let visibilities = Self::get_variable_visibilities(&shader_module);
        
        let variables = &shader_module.global_variables;
        for (handle, variable) in variables.iter() {
            let name = variable.name.as_ref()
                .ok_or("Global variable in shader does not have a name!")?;

            let binding = variable.binding.as_ref()
                .ok_or("Global variable in shader does not have a binding!")?;

            let visibility = *visibilities
                .get(&handle)
                .ok_or("Failed to find shader visibility for global variable!")?;

            let naga_type = naga_types.get_handle(variable.ty)
                .ok()
                .ok_or("Can't find type definition!!!")?
                .inner
                .clone();

            let binding_type = create_binding_type(&naga_type)
                .ok_or("Failed to translate naga type to binding type")?;
            
            let uniform = Uniform::new(binding.clone(), binding_type, visibility, naga_type);
            let should_be_none = uniforms.insert(name.clone(), uniform);
            if should_be_none.is_some() { 
                panic!("Defining same uniform name twice!");
            }
        }

        Ok(uniforms)
    }

    fn get_variable_visibilities(shader_module: &Module) -> HashMap<Handle<GlobalVariable>, wgpu::ShaderStages> {
        let entry_points = &shader_module.entry_points;
        // println!("{:#?}", entry_points);

        let mut visibilities = HashMap::new();

        for entry_point in entry_points {
            let stage = match &entry_point.stage {
                naga::ShaderStage::Vertex => wgpu::ShaderStages::VERTEX,
                naga::ShaderStage::Fragment => wgpu::ShaderStages::FRAGMENT,
                naga::ShaderStage::Compute => wgpu::ShaderStages::COMPUTE,
            };

            for (_, expr) in entry_point.function.expressions.iter() {
                if let Expression::GlobalVariable(handle) = expr {
                    visibilities.entry(handle.clone())
                        .and_modify(|s| *s = *s | stage)
                        .or_insert(stage);
                }
            }
        }   

        visibilities
    }

    fn correct_filterable_samplers(uniforms: &mut HashMap<String, Uniform>) {
        let mut sorted_by_group: HashMap<u32, Vec<&mut wgpu::BindingType>> = HashMap::new();

        for (_, uniform) in uniforms {
            let unif = sorted_by_group.entry(uniform.binding.group)
                .or_insert(vec![]);

            unif.push(&mut uniform.binding_type);
        }

        for group in sorted_by_group.values_mut() {
            if group.iter().find(|e| matches!(e, wgpu::BindingType::Texture { 
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    ..
                })).is_some()
            {
                //find the sampler attached to this object and set it to be filtering
                if let Some(sampler) = group.iter_mut().find(|e| matches!(e, wgpu::BindingType::Sampler(..))) {
                    **sampler = wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering)
                }
            }
        }

    }
}

const ATTRIBS: [wgpu::VertexAttribute; 1] =
        wgpu::vertex_attr_array![0 => Sint32];

impl Vertex for i32 {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        use std::mem;

        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &ATTRIBS,
        }
    }
}