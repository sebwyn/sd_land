use std::{borrow::Cow, collections::HashMap, num::NonZeroU64};

use legion::{World, IntoQuery};
use simple_error::SimpleError;
use uuid::Uuid;
use wgpu::{Instance, Surface, Adapter, Device, Queue, SurfaceConfiguration, Buffer, util::DeviceExt, RenderPipeline, BindGroup, BindGroupLayout};
use winit::{dpi::PhysicalSize, window::Window};

use crate::{graphics::{Rectangle, Vertex}, pipeline::Pipeline, material::Material, shader_types::MaterialValue};

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
        let mut query = <(&MaterialHandle, &Rectangle)>::query();

        let mut rects_by_material = HashMap::new();

        for (material, rect) in query.iter(world) {
            rects_by_material.entry(material.clone())
                .and_modify(|v: &mut Vec<&Rectangle>| v.push(rect))
                .or_insert(vec![rect]);
        }

        for (material, rect) in rects_by_material.iter() {
            { //update the bind groups if necessary
                let material_info = self.materials.get(material).unwrap();
                if material_info.dirty || material_info.bind_groups.is_none() {
                    let updated_bind_groups = self.create_bind_groups(material).unwrap();

                    drop(material_info);
                    let material_info = self.materials.get_mut(material).unwrap();
                    material_info.bind_groups = Some(updated_bind_groups);
                    material_info.dirty = false;
                }
            }

            let material_info = match self.materials.get(material) {
                Some(material) => material,
                None => continue,
            };

            //create all the vertices and all that
            let vertices = rect
                .iter()
                .map(|rect| rect.vertices)
                .flatten()
                .collect::<Vec<_>>();

            let num_rectangles = vertices.len() / 4;
            let indices = (0..num_rectangles)
                .into_iter()
                .map(|i| 
                    Rectangle::INDICES.iter()
                    .map(move |e| *e + (i * 4) as u32))
                .flatten()
                .collect::<Vec<_>>();

            let vertex_buffer = self.graphics.create_vertex_buffer(&vertices);
            let index_buffer = self.graphics.create_index_buffer(&indices);
            let num_indices = indices.len() as u32;

            let pipeline = &self.pipelines.get(&material_info.pipeline).as_ref().unwrap().1.pipeline;

            self.graphics.render(pipeline, material_info.bind_groups.as_ref().unwrap(), &vertex_buffer, &index_buffer, num_indices)?;
        }
        
        Ok(())
        // self.graphics.render(pipeline, vertex_buffer, index_buffer, num_indices)
    }

    pub fn find_display(&mut self) {
        self.graphics.resize(self.graphics.size());
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.graphics.resize(new_size);
    }

    pub fn create_texture(&mut self, file: &str) -> TextureHandle {
        let uuid = Uuid::new_v4();
        self.textures.insert(uuid, self.graphics.create_texture(file));
        uuid
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

    pub fn create_material(&mut self, pipeline_handle: PipelineHandle) -> Option<MaterialHandle> {
        let pipeline = &self.pipelines.get(&pipeline_handle).as_ref()?.0;
        let uuid = Uuid::new_v4();
        
        let cpu_storage = pipeline.new_material();
        let material_info = MaterialInfo {
            pipeline: pipeline_handle,
            cpu_storage: cpu_storage,
            bind_groups: None,
            dirty: true,
        };

        self.materials.insert(uuid, material_info);
        Some(uuid)
    }

    pub fn update_material<T: 'static>(&mut self, material_handle: MaterialHandle, name: &str, value: T) -> bool {
        if let Some(material) = self.materials.get_mut(&material_handle) {
            material.cpu_storage.set_uniform(name, value);
            material.dirty = true;
            true
        } else {
            false
        }
    }

    fn create_bind_groups(&self, material_handle: &Uuid) -> Result<Vec<wgpu::BindGroup>, SimpleError> {
        let material_info = self.materials.get(material_handle).unwrap();
        
        let uniforms = material_info.cpu_storage.uniforms();
        let bind_group_layouts = &self.pipelines.get(&material_info.pipeline)
            .as_ref()
            .ok_or(SimpleError::new("Could not find pipeline for material"))?
            .1.bind_group_layouts;
        
        let mut texture_views = HashMap::new();
        //generate the textures views that will be needed
        for (name, _, value) in uniforms.iter() {
            if let MaterialValue::Texture(texture) = value {
                let uuid = &texture.uuid
                    .ok_or(SimpleError::new(&format!("Could not find texture for material boud at: {}", name)))?;
                let texture_view = self.textures.get(uuid)
                    .ok_or(SimpleError::new(format!("Could not find texture in resources for uniform at: {}", name)))?
                    .create_view(&wgpu::TextureViewDescriptor::default());
            
                texture_views.insert(uuid.clone(), texture_view);
            }
        }

        //layouts, uniforms, textures, samplers
        self.graphics.create_bind_groups(bind_group_layouts, uniforms, &texture_views, &self.samplers)
    } 
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

pub struct Graphics {
    _instance: Instance,
    surface: Surface,
    _adapter: Adapter,
    device: Device,
    queue: Queue,
    config: SurfaceConfiguration,
    size: PhysicalSize<u32>,
}

impl Graphics {
    fn render(&mut self, 
        pipeline: &RenderPipeline,
        bind_groups: &[BindGroup], 
        vertex_buffer: &Buffer, 
        index_buffer: &Buffer, 
        num_indices: u32
    )  -> Result<(), wgpu::SurfaceError> {
        
        let output = self.surface.get_current_texture()?;
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());

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
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });

            render_pass.set_pipeline(pipeline);
            for (i, bind_group) in bind_groups.into_iter().enumerate() {
                render_pass.set_bind_group(i as u32, bind_group, &[]);
            }
            render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
            render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32); // 1.
            render_pass.draw_indexed(0..num_indices, 0, 0..1); // 2.
        }
    
        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
        }
    }
}


impl Graphics {
    fn size(&self) -> PhysicalSize<u32> { self.size }
}

impl Graphics {
fn create_texture(&self, file: &str) -> wgpu::Texture {

    let diffuse_bytes = std::fs::read(file).expect("Can't read texture file");
    let diffuse_image = image::load_from_memory(&diffuse_bytes).unwrap();
    let diffuse_rgba = diffuse_image.to_rgba8();

    use image::GenericImageView;
    let dimensions = diffuse_image.dimensions();

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
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
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
        &diffuse_rgba,
        wgpu::ImageDataLayout {
            offset: 0,
            bytes_per_row: std::num::NonZeroU32::new(4 * dimensions.0),
            rows_per_image: std::num::NonZeroU32::new(dimensions.1),
        },
        texture_size,
    );

    diffuse_texture
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
                byte_buffer.extend(bytes);
                groups.push((binding, bytes.len()));
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
                            resource: wgpu::BindingResource::TextureView(&texture_view),
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
                blend: Some(wgpu::BlendState::REPLACE),
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
        depth_stencil: None,
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
        backends: wgpu::Backends::all(),
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
        .copied()
        .filter(|f| f.describe().srgb)
        .next()
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

    Graphics {
        _instance: instance,
        surface,
        _adapter: adapter,
        device,
        queue,
        config,
        size,
    }
}
}