use std::collections::HashMap;

use naga::{ResourceBinding, Module, Handle, GlobalVariable, Expression};

use crate::shader_types::{MaterialValue, create_binding_type, create_uniform_storage};

pub struct Uniform {
    binding: ResourceBinding,
    binding_type: wgpu::BindingType,
    visibility: wgpu::ShaderStages,
    storage: MaterialValue
}

impl Uniform {
    fn new(binding: &ResourceBinding, binding_type: &wgpu::BindingType, visibility: &wgpu::ShaderStages, storage: &MaterialValue) -> Self {
        Self {
            binding: binding.clone(),
            binding_type: binding_type.clone(),
            visibility: visibility.clone(),
            storage: storage.clone()
        }
    }
}

pub trait Vertex {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a>;
}

pub struct Material {
    shader_source: String,
    uniforms: HashMap<String, Uniform>,
    vs_entry_point: String,
    fs_entry_point: String,
}

impl Material {
    fn load(shader: &str) -> Self {
        let shader_module = naga::front::wgsl::parse_str(shader).expect("Failed to load shader!");

        let vs_entry_point = shader_module.entry_points.iter()
            .find(|entry_point| entry_point.stage == naga::ShaderStage::Vertex)
            .expect("Could not find vertex program defined in shader!")
            .function.name
            .expect("Could not find name for vertex function!");

        let fs_entry_point = shader_module.entry_points.iter()
            .find(|entry_point| entry_point.stage == naga::ShaderStage::Fragment)
            .expect("Could not find vertex program defined in shader!")
            .function.name
            .expect("Could not find name for vertex function!");

        let uniforms = Self::parse_shader_uniforms(&shader_module);
        
        Self {
            shader_source: shader.to_string(),
            uniforms,
            vs_entry_point,
            fs_entry_point
        }
    }

    fn parse_shader_uniforms(shader_module: &Module) -> HashMap<String, Uniform> {
        let mut uniforms: HashMap<String, Uniform> = HashMap::new();
        
        let naga_types = &shader_module.types;

        let types = naga_types.iter()
            .map(|(handle, naga_type)| {
                (handle, create_binding_type(naga_type), create_uniform_storage(naga_type))
            })
            .collect::<Vec<_>>();

        let visibilities = Self::get_variable_visibilities(&shader_module);
        
        let variables = &shader_module.global_variables;
        for (handle, variable) in variables.iter() {
            let name = variable.name
                .as_ref()
                .expect("Global variable in shader does not have a name!");

            let binding = variable.binding
                .as_ref()
                .expect("Global variable in shader does not have a binding!");

            let visibility = visibilities
                .get(&handle)
                .expect("Failed to find shader visibility for global variable!");

            let (binding_type, storage) = types
                .iter()
                .find_map(|(handle, binding_type, storage)| 
                    if handle == &variable.ty { 
                        Some((binding_type, storage))
                    } else { 
                        None 
                    })
                .expect("Failed to find the handle for the global variable!!!!");
            
            let binding_type = binding_type.as_ref().expect("Failed to translate naga type to binding type");
            let storage = storage.as_ref().expect("Failed to create storage for naga type");

            let uniform = Uniform::new(binding, binding_type, visibility, storage);
            uniforms.insert(name.clone(), uniform);
        }

        uniforms
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
}

#[cfg(test)]
mod test {
    use crate::shader_types::{Texture, Sampler};

    use super::*;

    #[test]
    fn test_material_creation() {
        let shader = include_str!("shader.wgsl");
        let material = Material::load(shader);

        let uniforms = material.uniforms;

        let texture = uniforms.get("t_diffuse");
        let sampler = uniforms.get("s_diffuse");

        assert!(texture.is_some());
        assert!(sampler.is_some());
        let texture = texture.unwrap();
        let sampler = sampler.unwrap();

        assert!(matches!(texture.binding, ResourceBinding { group: 0, binding: 0 }));
        assert!(matches!(sampler.binding, ResourceBinding { group: 0, binding: 1 }));

        assert!(matches!(texture.visibility, wgpu::ShaderStages::FRAGMENT));
        assert!(matches!(sampler.visibility, wgpu::ShaderStages::FRAGMENT));

        assert!(matches!(
            texture.binding_type, 
            wgpu::BindingType::Texture { 
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                view_dimension: wgpu::TextureViewDimension::D2, 
                multisampled: false
            }
        ));
        assert!(matches!(
            sampler.binding_type, 
            wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering)
        ));

        assert!(matches!(
            texture.storage,
            MaterialValue::Texture(Texture {})
        ));
        assert!(matches!(
            sampler.storage,
            MaterialValue::Sampler(Sampler {})
        ));
    }

    #[test]
    fn test_get_variable_visibilities() {
        let shader = include_str!("shader.wgsl");
        let shader_module = naga::front::wgsl::parse_str(shader).unwrap();

        let handles = shader_module.global_variables
            .iter()
            .map(|(handle, _)| handle)
            .collect::<Vec<_>>();

        let visibilities = Material::get_variable_visibilities(&shader_module);

        for handle in handles {
            assert!(visibilities.contains_key(&handle));
            assert!(visibilities[&handle] == wgpu::ShaderStages::FRAGMENT)
        }
    }
}   