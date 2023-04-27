use crate::shader_types::MaterialValue;

pub struct Material {
    uniforms: Vec<(String, (u32, u32), MaterialValue)>,
}

impl Material {
    pub fn new(uniforms: Vec<(String, (u32, u32), MaterialValue)>) -> Self {
        Self {
            uniforms
        }
    }

    pub fn uniforms(&self) -> &Vec<(String, (u32, u32), MaterialValue)> {
        &self.uniforms
    }

    pub fn uniform<T: 'static>(mut self, name: &str, value: T) -> Self {
        self.set_uniform(name, value);
        self
    }

    pub fn set_uniform<T: 'static>(&mut self, name: &str, value: T) {
        if let Some((_, (..), current_value)) = self.uniforms.iter_mut()
            .find(|(uniform_name, _, _)| uniform_name == name) 
        {
            if let Some(current_value_t) = current_value.get_mut::<T>() {
                *current_value_t = value;
            } else {
                panic!("Shader definition of uniform ({name}) does not have that type!");
            }
        } else {
            panic!("Shader does not have uniform: {name}");
        }
    }

    pub fn get_uniform<T: 'static>(&self, name: &str) -> &T {
        if let Some((_, (..), current_value)) = self.uniforms.iter()
            .find(|(uniform_name, _, _)| uniform_name == name) 
        {
            if let Some(current_value_t) = current_value.get::<T>() {
                current_value_t
            } else {
                panic!("Shader definition of uniform ({name}) does not have that type!");
            }
        } else {
            panic!("Shader does not have uniform: {name}");
        }
    }
}