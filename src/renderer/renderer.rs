use std::{collections::HashMap, time::Instant, ops::Deref};
use core::fmt::Debug;

use image::ImageBuffer;
use legion::{World, Entity, IntoQuery, EntityStore};
use simple_error::SimpleError;
use uuid::Uuid;
use winit::window::Window;

use super::{pipeline::Pipeline, graphics::Graphics, graphics::{LoadedPipeline, RenderWork}, primitive::{Vertex, Visible, Rectangle}, view::{ViewRef, View}, camera::Camera, shader_types::{Matrix, MaterialValue}, material::Material};

pub struct MaterialInfo {
    pipeline: PipelineHandle,
    cpu_storage: Material,
    bind_groups: Option<Vec<wgpu::BindGroup>>,
    dirty: bool
}

pub struct Renderer {
    textures: HashMap<Uuid, wgpu::Texture>,
    samplers: HashMap<Uuid, wgpu::Sampler>,
    pipelines: HashMap<Uuid, (Pipeline, LoadedPipeline)>,
    materials: HashMap<Uuid, MaterialInfo>,

    graphics: Graphics
}


pub type TextureHandle = Uuid;
pub type SamplerHandle = Uuid;
pub type PipelineHandle = Uuid;
pub type MaterialHandle = Uuid;

impl Renderer {
    pub fn new(window: &Window) -> Self {
        let graphics = pollster::block_on(Graphics::new(window));

        Self {
            textures: HashMap::new(),
            samplers: HashMap::new(),
            pipelines: HashMap::new(),
            materials: HashMap::new(),
            graphics
        }
    }

    pub fn render(&mut self, world: &World) -> Result<(), wgpu::SurfaceError> {
        let _ = Instant::now();
        
        let mut viewed_elements = <(&Vec<Vertex>, &MaterialHandle, &ViewRef)>::query();

        //sort the elements by which view their in
        let mut elements_by_view: HashMap<Entity, HashMap<MaterialHandle, Vec<&Vec<Vertex>>>> = HashMap::new();
        for (vertices, material, view) in viewed_elements.iter(world) {
            let view = elements_by_view.entry(view.0)
                .or_insert(HashMap::new());
            
            let material_vec = view.entry(*material)
                .or_insert(Vec::new());

            material_vec.push(vertices);
        }

        self.graphics.begin_render([0f32, 0f32, 0f32])?;
        for (view, rects_by_material) in elements_by_view {
            //set the view
            let view_entity = world.entry_ref(view).expect("View entity does not exist!");
            let view = view_entity.get_component::<View>().expect("View entity doesn't have a view!");
            let camera = view_entity.get_component::<Camera>().expect("View entity doesn't have a camera!");
            
            let is_visible = view_entity.get_component::<Visible>().ok().is_some();
            if !is_visible {
                continue;
            }

            let view_proj_matrix = Matrix::from(camera.matrix());

            //udpate all the materials to have a camera
            for (material, _) in rects_by_material.iter() {
                //try and update the view_proj matrix, may fail, but that is fine
                self.update_material(*material, "view_proj", view_proj_matrix.clone());
                
                let material_info = self.materials.get_mut(material).unwrap();
                
                if material_info.dirty || material_info.bind_groups.is_none() {
                    let updated_bind_groups = self.create_bind_groups(material).unwrap();
    
                    let material_info = self.materials.get_mut(material).unwrap();
                    material_info.bind_groups = Some(updated_bind_groups);
                    material_info.dirty = false;
                }
            }

            let mut render_tasks = Vec::new();
            for (material, vertex_sets) in rects_by_material.iter() {

                let material_info = match self.materials.get(material) {
                    Some(material) => material,
                    None => continue,
                };

                let vertices: Vec<Vertex> = 
                    vertex_sets.iter().flat_map(|v| (*v).clone()).collect::<Vec<_>>();
                    
                let num_rectangles = vertices.len() / 4;
                let indices = (0..num_rectangles)
                    .flat_map(|i| 
                        Rectangle::INDICES.iter()
                        .map(move |e| *e + (i * 4) as u32))
                    .collect::<Vec<_>>();

                let vertex_buffer = self.graphics.create_vertex_buffer(&vertices);
                let index_buffer = self.graphics.create_index_buffer(&indices);
                let num_indices = indices.len() as u32;

                let pipeline = &self.pipelines.get(&material_info.pipeline).as_ref().unwrap().1.pipeline;

                render_tasks.push(RenderWork {
                    pipeline, 
                    bind_groups: material_info.bind_groups.as_ref().unwrap(), 
                    vertex_buffer, 
                    index_buffer, 
                    num_indices,
                    view: Some(view),
                });
            }

            self.graphics.render(render_tasks)?;
        }
        self.graphics.flush();

        Ok(())

    }

    pub fn find_display(&mut self) {
        self.graphics.resize(self.graphics.size());
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.graphics.resize(new_size);
    }

    pub fn load_texture(&mut self, file: &str) -> Result<TextureHandle, SimpleError> {
        let uuid = Uuid::new_v4();

        let diffuse_bytes = std::fs::read(file).expect("Can't read texture file");
        let diffuse_image = image::load_from_memory(&diffuse_bytes).unwrap();
        let diffuse_rgba = diffuse_image.to_rgba8();

        self.textures.insert(uuid, self.graphics.create_texture(diffuse_rgba)?);
        Ok(uuid)
    }

    pub fn create_texture<P, S>(&mut self, image: ImageBuffer<P, S>) -> Result<TextureHandle, SimpleError> 
    where 
        P: image::Pixel<Subpixel = u8>,
        S: Deref<Target = [<P as image::Pixel>::Subpixel]>,
    {
        let uuid = Uuid::new_v4();
        self.textures.insert(uuid, self.graphics.create_texture(image)?);
        Ok(uuid)
    }

    pub fn create_sampler(&mut self, ) -> SamplerHandle {
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

    pub fn update_material<T>(&mut self, material_handle: MaterialHandle, name: &str, value: T) -> bool 
        where T: 'static + Debug
    {
        if let Some(material) = self.materials.get_mut(&material_handle) {
            if material.cpu_storage.set_uniform(name, value) {
                material.dirty = true;
                return true;
            }   
        }
        false
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