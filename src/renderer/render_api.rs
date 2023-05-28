use std::{collections::HashMap, ops::Deref};
use core::fmt::Debug;

use image::ImageBuffer;
use simple_error::SimpleError;
use uuid::Uuid;
use winit::{window::Window, dpi::PhysicalSize};

use super::{
    pipeline::Pipeline, 
    graphics::Graphics, 
    graphics::{LoadedPipeline, GraphicsWork}, 
    view::View,
    shader_types::MaterialValue, 
    material::Material
};

pub struct MaterialInfo {
    pipeline: PipelineHandle,
    cpu_storage: Material,
    bind_groups: Option<Vec<wgpu::BindGroup>>,
    dirty: bool
}

pub struct RenderApi {
    textures: HashMap<Uuid, wgpu::Texture>,
    samplers: HashMap<Uuid, wgpu::Sampler>,
    pipelines: HashMap<Uuid, (Pipeline, LoadedPipeline)>,
    materials: HashMap<Uuid, MaterialInfo>,

    graphics: Graphics,
}

pub struct RenderWork<T, I> {
    pub vertices: Vec<T>,
    pub indices: Vec<u32>,
    pub instances: Option<Vec<I>>,
    pub material: MaterialHandle
}

pub type TextureHandle = Uuid;
pub type SamplerHandle = Uuid;
pub type PipelineHandle = Uuid;
pub type MaterialHandle = Uuid;

impl RenderApi {
    pub fn new(window: &Window) -> Self {
        let graphics = pollster::block_on(Graphics::new(window));

        Self {
            textures: HashMap::new(),
            samplers: HashMap::new(),
            pipelines: HashMap::new(),
            materials: HashMap::new(),
            graphics,
        }
    }

    pub fn screen_size(&self) -> (u32, u32) {
        (self.graphics.size().width, self.graphics.size().height)
    }

    pub fn begin_render(&mut self) -> Result<(), wgpu::SurfaceError> { self.graphics.begin_render()?; Ok(()) }
    pub fn flush(&mut self) { self.graphics.flush(); }

    pub fn submit_subrender<T, I>(&mut self, work: &[RenderWork<T, I>], view: Option<&View>)
        -> Result<(), wgpu::SurfaceError> 
    where
        T: bytemuck::Pod,
        I: bytemuck::Pod
    {
        self.graphics.clear_depth()?;

        for RenderWork { vertices, indices, instances, material } in work {
            let vertex_buffer = self.graphics.create_vertex_buffer(vertices);
            let index_buffer = self.graphics.create_index_buffer(indices);
            let num_indices = indices.len() as u32;

            let instance_buffer = instances.as_ref().map(|instances| self.graphics.create_instance_buffer(instances));
            let num_instances = instances.as_ref().map(|instances| instances.len() as u32);

            {
                let material_info = match self.materials.get(material) {
                    Some(info) => info,
                    None => continue,
                };

                if material_info.dirty || material_info.bind_groups.is_none() {
                    let new_bind_groups = Some(self.create_bind_groups(material).unwrap());
                    
                    let material_info = self.materials.get_mut(material).unwrap();
                    material_info.bind_groups = new_bind_groups;
                    material_info.dirty = false;
                }
            }

            let material_info = self.materials.get(material).unwrap();

            let pipeline = match self.pipelines.get(&material_info.pipeline) {
                Some(pipeline) => &pipeline.1.pipeline,
                None => continue
            };

            self.graphics.render(vec![GraphicsWork {
                pipeline,
                bind_groups: material_info.bind_groups.as_ref().unwrap(),
                vertex_buffer,
                index_buffer,
                num_indices,
                instance_buffer,
                num_instances,
                view,
            }])?;
        }
        Ok(())
    }

    pub fn find_display(&mut self) {
        self.graphics.resize(self.graphics.size());
    }

    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        self.graphics.resize(new_size);
    }

    pub fn load_texture(&mut self, file: &str) -> Result<TextureHandle, SimpleError> {
        let uuid = Uuid::new_v4();

        let diffuse_bytes = std::fs::read(file).expect("Can't read texture file");
        let diffuse_image = image::load_from_memory(&diffuse_bytes).unwrap();
        let diffuse_rgba = diffuse_image.to_rgba8();

        self.textures.insert(uuid, self.graphics.create_texture(&diffuse_rgba)?);
        Ok(uuid)
    }

    pub fn create_texture<P, S>(&mut self, image: &ImageBuffer<P, S>) -> Result<TextureHandle, SimpleError> 
    where 
        P: image::Pixel<Subpixel = u8>,
        S: Deref<Target = [<P as image::Pixel>::Subpixel]>,
    {
        let uuid = Uuid::new_v4();
        self.textures.insert(uuid, self.graphics.create_texture(image)?);
        Ok(uuid)
    }

    pub fn create_sampler(&mut self) -> SamplerHandle {
        let uuid = Uuid::new_v4();
        self.samplers.insert(uuid, self.graphics.create_sampler());
        uuid
    }

    pub fn create_pipeline(&mut self, pipeline: Pipeline) -> PipelineHandle {
        let uuid = Uuid::new_v4();
        self.pipelines.insert(uuid, (pipeline.clone(), self.graphics.load_pipeline(pipeline)));
        uuid
    }

    pub fn create_material(&mut self, pipeline_handle: PipelineHandle) -> Result<MaterialHandle, SimpleError> {
        let pipeline = &self.pipelines.get(&pipeline_handle)
            .as_ref()
            .ok_or(SimpleError::new("Could not find pipeline to create material from!"))?.0;
        let uuid = Uuid::new_v4();
        
        let cpu_storage = pipeline.new_material();
        let material_info = MaterialInfo {
            pipeline: pipeline_handle,
            cpu_storage,
            bind_groups: None,
            dirty: true,
        };

        self.materials.insert(uuid, material_info);
        Ok(uuid)
    }

    pub fn update_material<T>(&mut self, material_handle: MaterialHandle, name: &str, value: T) -> Result<(), SimpleError> 
        where T: 'static + Debug
    {
        if let Some(material) = self.materials.get_mut(&material_handle) {
            if material.cpu_storage.set_uniform(name, value) {
                material.dirty = true;
                return Ok(())
            }   
        }
        Err(SimpleError::new("Material either does not have that uniform or it is the wrong type"))
    }

    fn create_bind_groups(&self, material_handle: &Uuid) -> Result<Vec<wgpu::BindGroup>, SimpleError> {
        let material_info = self.materials.get(material_handle).unwrap();
        
        let uniforms = material_info.cpu_storage.uniforms();
        let bind_group_layouts = &self.pipelines.get(&material_info.pipeline)
            .as_ref()
            .ok_or(SimpleError::new("Could not find pipeline for material"))?
            .1.bind_group_layouts;
        
        let mut texture_views = HashMap::new();
        for (name, _, value) in uniforms.iter() {
            if let MaterialValue::Texture(texture) = value {
                let uuid = &texture.uuid
                    .ok_or(SimpleError::new(&format!("Could not find texture for material bound at: {}", name)))?;
                let texture_view = self.textures.get(uuid)
                    .ok_or(SimpleError::new(format!("Could not find texture in resources for uniform at: {}", name)))?
                    .create_view(&wgpu::TextureViewDescriptor::default());
            
                texture_views.insert(*uuid, texture_view);
            }
        }

        //layouts, uniforms, textures, samplers
        self.graphics.create_bind_groups(bind_group_layouts, uniforms, &texture_views, &self.samplers)
    } 
}