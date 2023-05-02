use std::{borrow::Cow, collections::HashMap, num::NonZeroU64, ops::Deref, time::Instant};
use core::fmt::Debug;

use image::ImageBuffer;
use legion::{World, Entity, EntityStore, IntoQuery};
use simple_error::SimpleError;
use uuid::Uuid;
use wgpu::{Instance, Surface, Adapter, Device, Queue, SurfaceConfiguration, Buffer, util::DeviceExt, RenderPipeline, BindGroup, BindGroupLayout, CommandBuffer, SurfaceTexture, SurfaceError};
use winit::{dpi::PhysicalSize, window::Window};

use crate::{graphics::{Rectangle, Vertex, Visible}, pipeline::Pipeline, material::Material, shader_types::{MaterialValue, Matrix}, camera::Camera, view::{View, ViewRef}};

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
        
        let mut viewed_elements = <(&Rectangle, &MaterialHandle, &RenderStage, &ViewRef)>::query();

        //sort the elements by which view their in
        let mut elements_by_view: HashMap<Entity, HashMap<MaterialHandle, Vec<&Rectangle>>> = HashMap::new();
        for (rectangle, material, _, view, ..) in viewed_elements.iter(world) {
            let view = elements_by_view.entry(view.0)
                .or_insert(HashMap::new());
            
            let material_vec = view.entry(*material)
                .or_insert(Vec::new());

            material_vec.push(rectangle);
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
            for (material, rectangles) in rects_by_material.iter() {
                
                // println!("{}\n{:#?}", material, rectangles);

                let material_info = match self.materials.get(material) {
                    Some(material) => material,
                    None => continue,
                };

                let vertices: Vec<Vertex> = rectangles
                    .iter()
                    .filter(|rect| camera.is_visible(&rect.vertices))
                    .flat_map(|rect| rect.vertices)
                    .collect();

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

pub struct RenderStage {
    pub order: u32,
}

struct MaterialInfo {
    pipeline: PipelineHandle,
    cpu_storage: Material,
    bind_groups: Option<Vec<wgpu::BindGroup>>,
    dirty: bool
}

struct LoadedPipeline {
    pipeline: RenderPipeline, 
    bind_group_layouts: Vec<(u32, BindGroupLayout)>,
}

pub struct RenderWork<'a> {
    pipeline: &'a RenderPipeline,
    bind_groups: &'a [BindGroup], 
    vertex_buffer: Buffer, 
    index_buffer: Buffer, 
    num_indices: u32,
    view: Option<&'a View>,
}

pub struct Graphics {
    _instance: Instance,
    surface: Surface,
    _adapter: Adapter,
    device: Device,
    queue: Queue,
    config: SurfaceConfiguration,
    size: PhysicalSize<u32>,
    depth_texture: (wgpu::Texture, wgpu::TextureView, wgpu::Sampler),

    current_surface_texture: Option<SurfaceTexture>,
    command_buffers: Vec<CommandBuffer>, 
}

impl Graphics {
    fn begin_render(&mut self, clear_color: [f32; 3]) -> Result<(), SurfaceError>{
        let output = self.surface.get_current_texture()?;
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        {
            encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: clear_color[0] as f64,
                            g: clear_color[1] as f64,
                            b: clear_color[2] as f64,
                            a: 1.0,
                        }),
                        store: true,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_texture.1,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: true,
                    }),
                    stencil_ops: None,
                }),
            });
        }

        self.command_buffers.push(encoder.finish());
        self.current_surface_texture.replace(output);

        Ok(())
    }

    fn render(&mut self, 
        work: Vec<RenderWork>,
    )  -> Result<(), wgpu::SurfaceError> {
        
        let view = self.current_surface_texture.as_ref()
            .expect("Render must be called after starting to render")
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: true,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_texture.1,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: true,
                    }),
                    stencil_ops: None,
                }),
            });

            for task in work.iter() {
                if let Some(view) = &task.view {
                    // println!("{}, {}", view.x_pos(self.config.width), view.y_pos(self.config.height));
                    // println!("{}, {}", view.width(self.config.width), view.height(self.config.height));
                    
                    render_pass.set_viewport(
                        view.x_pos(), 
                        view.y_pos(), 
                        view.width(), 
                        view.height(),
                        view.near(),
                        view.far() 
                    )
                } else {
                    //set the viewport to be the full screen
                    render_pass.set_viewport(-1.0, -1.0, 2.0, 2.0, 0.0, 1.0);
                }
                
                render_pass.set_pipeline(task.pipeline);

                for (i, bind_group) in task.bind_groups.iter().enumerate() {
                    render_pass.set_bind_group(i as u32, bind_group, &[]);
                }
                render_pass.set_vertex_buffer(0, task.vertex_buffer.slice(..));
                render_pass.set_index_buffer(task.index_buffer.slice(..), wgpu::IndexFormat::Uint32); // 1.
                render_pass.draw_indexed(0..task.num_indices, 0, 0..1); // 2.     
            }
        }

        self.command_buffers.push(encoder.finish());

        Ok(())
    }

    fn flush(&mut self) {
        let command_buffers = self.command_buffers.drain(0..).collect::<Vec<_>>();
        self.queue.submit(command_buffers);

        let surface_texture = self.current_surface_texture.take()
            .expect("Must call begin render before flush");

        surface_texture.present();
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
            self.depth_texture = Self::create_depth_texture(&self.device, &self.config, "Some depth texture");
        }
    }
}


impl Graphics {
    fn size(&self) -> PhysicalSize<u32> { self.size }
}

impl Graphics {
fn create_texture<P, S>(&self, image: ImageBuffer<P, S>) -> Result<wgpu::Texture, SimpleError>
where 
    P: image::Pixel<Subpixel = u8>,
    S: Deref<Target = [<P as image::Pixel>::Subpixel]>,
{
    let format = match P::CHANNEL_COUNT {
        1 => wgpu::TextureFormat::R8Unorm,
        4 => wgpu::TextureFormat::Rgba8UnormSrgb,
        _ => return Err(SimpleError::new("Could not create texture of that format!"))
    };

    let dimensions = image.dimensions();

    let texture_size = wgpu::Extent3d {
        width: dimensions.0,
        height: dimensions.1,
        depth_or_array_layers: 1,
    };

    let diffuse_texture = self.device.create_texture(
        &wgpu::TextureDescriptor {
            size: texture_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            label: Some("diffuse_texture"),
            view_formats: &[],
        }
    );

    self.queue.write_texture(
        wgpu::ImageCopyTexture {
            texture: &diffuse_texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &image,
        wgpu::ImageDataLayout {
            offset: 0,
            bytes_per_row: std::num::NonZeroU32::new(P::CHANNEL_COUNT as u32 * dimensions.0),
            rows_per_image: std::num::NonZeroU32::new(dimensions.1),
        },
        texture_size,
    );

    Ok(diffuse_texture)
}

fn create_sampler(&self) -> wgpu::Sampler {
    self.device.create_sampler(&wgpu::SamplerDescriptor {
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Nearest,
        mipmap_filter: wgpu::FilterMode::Nearest,
        ..Default::default()
    })
}

fn create_bind_groups(&self, 
        bind_group_layouts: &[(u32, wgpu::BindGroupLayout)], 
        uniforms: &[(String, (u32, u32), MaterialValue)], 
        textures: &HashMap<Uuid, wgpu::TextureView>,
        samplers: &HashMap<Uuid, wgpu::Sampler>,
    ) -> Result<Vec<wgpu::BindGroup>, SimpleError> {
    let mut bind_groups = Vec::new();
    for group_index in 0.. {
        let bind_group_layout = bind_group_layouts.iter().find(|(index, _)| group_index == *index);
        if bind_group_layout.is_none() {
            break;
        }
        let bind_group_layout = bind_group_layout.unwrap();

        //create the offsets here for each group
        let mut groups = Vec::new();
        let mut byte_buffer: Vec<u8> = Vec::new();
        for (_, (group, binding), value) in uniforms {
            if *group != group_index {
                continue
            }
            
            if let Some(bytes) = value.as_bytes() {
                groups.push((binding, bytes.len()));
                byte_buffer.extend(bytes);
            }
        }
        let buffer = self.create_uniform_buffer(&byte_buffer);

        let mut offset = 0;
        let mut entries = Vec::new();
        for (name, (group, binding), value) in uniforms {
            if *group != group_index {
                continue
            }
            
            let entry =
            if let Some((_, size)) = groups.iter()
                .find(|(groups_binding, _)| *groups_binding == binding)
            {
                let size = *size as u64;
                let entry = wgpu::BindGroupEntry {
                    binding: *binding,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &buffer,
                        offset,
                        size: NonZeroU64::new(size),
                    })
                };
                offset += size;
                entry
            } else {
                match value {
                    crate::shader_types::MaterialValue::Texture(texture) => {
                        let uuid = &texture.uuid
                            .ok_or(SimpleError::new(&format!("Material was never assigned texture at: {}", name)))?;
                        
                        let texture_view = textures.get(uuid)
                            .ok_or(SimpleError::new(&format!("Cannot find texture assigned to material at: {}", name)))?;
                        
                        wgpu::BindGroupEntry {
                            binding: *binding,
                            resource: wgpu::BindingResource::TextureView(texture_view),
                        }
                    },
                    crate::shader_types::MaterialValue::Sampler(sampler) => {
                        let sampler = samplers.get(
                                &sampler.uuid
                                .ok_or(SimpleError::new(&format!("Material was never assigned sampler: {}", name)))?
                            )
                            .ok_or(SimpleError::new(&format!("Cannot find sampler assigned to material at: {}", name)))?;

                        wgpu::BindGroupEntry {
                            binding: *binding,
                            resource: wgpu::BindingResource::Sampler(sampler)
                        }
                    },
                    _ => panic!("Can't create bind group entry for uniform {}", name)
                }
            };
            entries.push(entry);
        }

        let binding_descriptor = wgpu::BindGroupDescriptor {
            label: Some("Some bind group"),
            layout: &bind_group_layout.1,
            entries: &entries,
        };

        bind_groups.push(self.device.create_bind_group(&binding_descriptor));
    }

    Ok(bind_groups)
}

fn load_pipeline(&mut self, pipeline: Pipeline) -> LoadedPipeline {
    let material_bind_groups = pipeline.bind_groups();

    let mut group_index = 0;
    let mut group_and_bind_group_layouts = Vec::new();
    for group in material_bind_groups {
        let mut entries = Vec::new();
        for layout in group {
            group_index = layout.binding.group;
            entries.push(wgpu::BindGroupLayoutEntry {
                binding: layout.binding.binding,
                visibility: layout.visibility,
                ty: layout.binding_type,
                count: None,
            })
        }

        group_and_bind_group_layouts.push((group_index, self.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &entries,
            label: Some("texture_bind_group_layout"),
        })));
    }

    group_and_bind_group_layouts.sort_by(|(group_1, _), (group_2, _)| group_1.cmp(group_2));

    let bind_group_layouts = group_and_bind_group_layouts.iter().map(|(_, group)| group).collect::<Vec<_>>();

    let render_pipeline_layout =
        self.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: bind_group_layouts.as_slice(),
            push_constant_ranges: &[],
        });
    
    let shader = self.device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Shader"),
        source: wgpu::ShaderSource::Wgsl(Cow::from(pipeline.shader())),
    });
    
    let render_pipeline = self.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Render Pipeline"),
        layout: Some(&render_pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: pipeline.vs_entry_point(),
            buffers: pipeline.buffer_layouts(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: pipeline.fs_entry_point(),

            //TODO: implement in material
            targets: &[Some(wgpu::ColorTargetState {
                format: self.config.format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),

        //TODO: implement in material
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: Some(wgpu::Face::Back),
            polygon_mode: wgpu::PolygonMode::Fill,
            unclipped_depth: false,
            conservative: false,
        },

        //TODO: implement in material
        depth_stencil: Some(wgpu::DepthStencilState {
            format: Self::DEPTH_FORMAT,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::Less, // 1.
            stencil: wgpu::StencilState::default(), // 2.
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState {
            count: 1,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        multiview: None,
    });

    LoadedPipeline { pipeline: render_pipeline, bind_group_layouts: group_and_bind_group_layouts }
}

fn create_vertex_buffer(&self, vertices: &[Vertex]) -> Buffer {
    self.device.create_buffer_init(
        &wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(vertices),
            usage: wgpu::BufferUsages::VERTEX,
        }
    )
}

fn create_index_buffer(&self, indices: &[u32]) -> Buffer {
    self.device.create_buffer_init(
        &wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(indices),
            usage: wgpu::BufferUsages::INDEX,
        }
    )
}

fn create_uniform_buffer(&self, bytes: &[u8]) -> Buffer {
    self.device.create_buffer_init(
        &wgpu::util::BufferInitDescriptor {
            label: Some("Uniform buffer"),
            contents: bytes,
            usage: wgpu::BufferUsages::UNIFORM,
        }
    )
}

async fn new(window: &Window) -> Graphics {
    let size = window.inner_size();

    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::METAL,
        dx12_shader_compiler: Default::default(),
    });
    
    let surface = unsafe { instance.create_surface(&window) }.unwrap();

    let adapter = instance.request_adapter(
        &wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        },
    ).await.unwrap();

    let (device, queue) = adapter.request_device(
        &wgpu::DeviceDescriptor {
            features: wgpu::Features::empty(),
            // WebGL doesn't support all of wgpu's features, so if
            // we're building for the web we'll have to disable some.
            limits: if cfg!(target_arch = "wasm32") {
                wgpu::Limits::downlevel_webgl2_defaults()
            } else {
                wgpu::Limits::default()
            },
            label: None,
        },
        None, // Trace path
    ).await.unwrap();

    let surface_caps = surface.get_capabilities(&adapter);

    let surface_format = surface_caps.formats.iter()
        .find(|f| f.describe().srgb)
        .cloned()
        .unwrap_or(surface_caps.formats[0]);

    let config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: surface_format,
        width: size.width,
        height: size.height,
        present_mode: surface_caps.present_modes[0],
        alpha_mode: surface_caps.alpha_modes[0],
        view_formats: vec![],
    };
    surface.configure(&device, &config);

    let depth_texture = Self::create_depth_texture(&device, &config, "Some depth texture");

    Graphics {
        _instance: instance,
        surface,
        _adapter: adapter,
        device,
        queue,
        config,
        size,
        depth_texture,
        current_surface_texture: None,
        command_buffers: Vec::new()
    }
}

const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float; // 1.
    
fn create_depth_texture(device: &wgpu::Device, config: &wgpu::SurfaceConfiguration, label: &str) 
-> (wgpu::Texture, wgpu::TextureView, wgpu::Sampler) 
{
    let size = wgpu::Extent3d { // 2.
        width: config.width,
        height: config.height,
        depth_or_array_layers: 1,
    };
    let desc = wgpu::TextureDescriptor {
        label: Some(label),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: Self::DEPTH_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT // 3.
            | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    };
    let texture = device.create_texture(&desc);

    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let sampler = device.create_sampler(
        &wgpu::SamplerDescriptor { // 4.
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            compare: Some(wgpu::CompareFunction::LessEqual), // 5.
            lod_min_clamp: 0.0,
            lod_max_clamp: 100.0,
            ..Default::default()
        }
    );

    (texture, view, sampler)
}

}