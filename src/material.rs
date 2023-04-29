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

    pub fn set_uniform<T: 'static>(&mut self, name: &str, value: T) -> bool{
        if let Some((_, (..), current_value)) = self.uniforms.iter_mut()
            .find(|(uniform_name, _, _)| uniform_name == name) 
        {
            if let Some(current_value_t) = current_value.get_mut::<T>() {
                *current_value_t = value;
                true
            } else {
                false
            }
        } else {
            false
        }
    }

    pub fn get_uniform<T: 'static>(&self, name: &str) -> Option<&T> {
        if let Some((_, (..), current_value)) = self.uniforms.iter()
            .find(|(uniform_name, _, _)| uniform_name == name) 
        {
            if let Some(current_value_t) = current_value.get::<T>() {
                Some(current_value_t)
            } else {
                None
            }
        } else {
            None
        }
    }
}