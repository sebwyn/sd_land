use std::{borrow::Cow, collections::HashMap, num::NonZeroU64, ops::Deref};

use image::ImageBuffer;

use simple_error::SimpleError;
use uuid::Uuid;
use wgpu::{Instance, Surface, Adapter, Device, Queue, SurfaceConfiguration, Buffer, util::DeviceExt, RenderPipeline, BindGroup, BindGroupLayout, CommandBuffer, SurfaceTexture, SurfaceError};
use winit::{dpi::PhysicalSize, window::Window};

use super::{primitive::Vertex, pipeline::Pipeline, shader_types::MaterialValue, view::View};

pub struct RenderStage {
    pub order: u32,
}

pub struct LoadedPipeline {
    pub(super) pipeline: RenderPipeline, 
    pub(super) bind_group_layouts: Vec<(u32, BindGroupLayout)>,
}

pub struct RenderWork<'a> {
    pub(super) pipeline: &'a RenderPipeline,
    pub(super) bind_groups: &'a [BindGroup], 
    pub(super) vertex_buffer: Buffer, 
    pub(super) index_buffer: Buffer, 
    pub(super) num_indices: u32,
    pub(super) view: Option<&'a View>,
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
    pub(super) fn begin_render(&mut self, clear_color: [f32; 3]) -> Result<(), SurfaceError>{
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

    pub(super) fn render(&mut self, 
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

    pub(super) fn flush(&mut self) {
        let command_buffers = self.command_buffers.drain(0..).collect::<Vec<_>>();
        self.queue.submit(command_buffers);

        let surface_texture = self.current_surface_texture.take()
            .expect("Must call begin render before flush");

        surface_texture.present();
    }

    pub(super) fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
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
    pub(super) fn size(&self) -> PhysicalSize<u32> { self.size }
}

impl Graphics {
pub(super) fn create_texture<P, S>(&self, image: ImageBuffer<P, S>) -> Result<wgpu::Texture, SimpleError>
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

pub(super) fn create_sampler(&self) -> wgpu::Sampler {
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

pub(super) fn create_bind_groups(&self, 
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
                    super::shader_types::MaterialValue::Texture(texture) => {
                        let uuid = &texture.uuid
                            .ok_or(SimpleError::new(&format!("Material was never assigned texture at: {}", name)))?;
                        
                        let texture_view = textures.get(uuid)
                            .ok_or(SimpleError::new(&format!("Cannot find texture assigned to material at: {}", name)))?;
                        
                        wgpu::BindGroupEntry {
                            binding: *binding,
                            resource: wgpu::BindingResource::TextureView(texture_view),
                        }
                    },
                    super::shader_types::MaterialValue::Sampler(sampler) => {
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

pub(super) fn load_pipeline(&mut self, pipeline: Pipeline) -> LoadedPipeline {
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

pub(super) fn create_vertex_buffer(&self, vertices: &[Vertex]) -> Buffer {
    self.device.create_buffer_init(
        &wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(vertices),
            usage: wgpu::BufferUsages::VERTEX,
        }
    )
}

pub(super) fn create_index_buffer(&self, indices: &[u32]) -> Buffer {
    self.device.create_buffer_init(
        &wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(indices),
            usage: wgpu::BufferUsages::INDEX,
        }
    )
}

pub(super) fn create_uniform_buffer(&self, bytes: &[u8]) -> Buffer {
    self.device.create_buffer_init(
        &wgpu::util::BufferInitDescriptor {
            label: Some("Uniform buffer"),
            contents: bytes,
            usage: wgpu::BufferUsages::UNIFORM,
        }
    )
}

pub(super) async fn new(window: &Window) -> Graphics {
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