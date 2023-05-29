use std::collections::HashMap;
use legion::{component, IntoQuery, Query, system};
use legion::systems::Builder;
use legion::world::SubWorld;
use simple_error::SimpleError;
use crate::layout::Transform;
use crate::renderer::camera::Camera;
use crate::renderer::pipeline::Pipeline;
use crate::renderer::primitive::{Rectangle, RectangleVertex};
use crate::renderer::render_api::{MaterialHandle, RenderApi, RenderWork};
use crate::renderer::shader_types::{Matrix, Sampler, Texture};

pub struct ActiveSceneCamera;

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Image {
    image_path: String,
    smooth_sampling: bool,
}

impl Image {
    pub fn new(image_path: &str, smooth_sampling: bool) -> Self {
        Self {
            image_path: image_path.to_string(),
            smooth_sampling,
        }
    }

    pub fn image_path(&self) -> &str {
        &self.image_path
    }

    pub fn tex_coords(&self) -> ([f32; 2], [f32; 2]) {
        ([0f32; 2], [1f32; 2])
    }
}

#[derive(Hash, Clone, PartialEq, Eq)]
pub struct SpriteSheetSprite {
    //sprite_sheet view
    width: u32,
    height: u32,

    //sprite_sheet_position
    tile_x: u32,
    tile_y: u32,
}

impl SpriteSheetSprite {
    pub fn from_sprite_sheet_dimensions(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            tile_x: 0,
            tile_y: 0,
        }
    }

    pub fn set_tile(&mut self, tile_x: u32, tile_y: u32) {
        self.tile_x = tile_x;
        self.tile_y = tile_y;
    }

    pub fn tex_coords(&self) -> ([f32; 2], [f32; 2]) {
        let tile_x = self.tile_x.clamp(0, self.width - 1);
        let tile_y = self.tile_y.clamp(0, self.height - 1);

        let tex_dimensions = [1.0 / self.width as f32, 1.0 / self.height as f32];
        let tex_origin = [tile_x as f32 * tex_dimensions[0], tile_y as f32 * tex_dimensions[1]];

        (tex_origin, tex_dimensions)
    }
}

pub struct SpriteRenderer {
    images: HashMap<String, Texture>,
    material: MaterialHandle,

    rough_sampler: Sampler,
    smooth_sampler: Sampler,
}

impl SpriteRenderer {
    pub fn new(render_api: &mut RenderApi) -> Result<Self, SimpleError> {
        let pipeline = Pipeline::load(include_str!("shaders/sprite.wgsl"))?
            .with_vertex::<RectangleVertex>()
            .with_instance::<Rectangle>();

        let pipeline_handle = render_api.create_pipeline(pipeline);

        let material = render_api.create_material(pipeline_handle)?;

        let rough_sampler = Sampler::new(render_api.create_sampler(wgpu::FilterMode::Nearest));
        let smooth_sampler = Sampler::new(render_api.create_sampler(wgpu::FilterMode::Linear));


        Ok(Self {
            images: HashMap::new(),
            material,
            rough_sampler,
            smooth_sampler,
        })
    }
}

pub fn add_sprite_subrender(sprite_renderer: SpriteRenderer, schedule: &mut Builder) { schedule.add_system(render_sprites_system(sprite_renderer)); }

#[system]
#[read_component(Camera)]
#[read_component(ActiveSceneCamera)]
fn render_sprites(
    #[state] sprite_storage: &mut SpriteRenderer,
    world: &SubWorld,
    sprite_query: &mut Query<(&Image, &Transform, Option<&SpriteSheetSprite>)>,
    #[resource] render_api: &mut RenderApi,
) {
    let active_camera = <&Camera>::query().filter(component::<ActiveSceneCamera>()).iter(world).next().unwrap();
    let scene_view_proj_matrix = Matrix::from(active_camera.matrix());
    render_api.update_material(sprite_storage.material, "view_proj", scene_view_proj_matrix).unwrap();

    struct SpriteOptions<'a> {
        transform: &'a Transform,
        tex_coords: ([f32; 2], [f32; 2]),
    }

    let mut sprites_by_image = HashMap::new();

    for (image, transform, sprite_sheet_sprite) in sprite_query.iter(world) {
        if transform.visible {
            let sprites = sprites_by_image.entry(image.clone())
                .or_insert(Vec::new());

            if let Some(sprite_sheet_sprite) = sprite_sheet_sprite {
                let tex_coords = sprite_sheet_sprite.tex_coords();
                sprites.push(SpriteOptions { transform, tex_coords });
            } else {
                sprites.push(SpriteOptions { transform, tex_coords: image.tex_coords() });
            }
        }
    }

    let vertices = Rectangle::VERTICES.to_vec();
    let indices = Rectangle::INDICES.to_vec();

    for (image, sprites) in sprites_by_image {
        let texture = &*sprite_storage.images.entry(image.image_path().to_string())
            .or_insert_with(|| {
                //load the image
                Texture::new(render_api.load_texture(image.image_path()).unwrap())
            });

        let sampler = if image.smooth_sampling { &sprite_storage.smooth_sampler } else { &sprite_storage.rough_sampler }.clone();

        render_api.update_material(sprite_storage.material, "t_diffuse", texture.clone()).unwrap();
        render_api.update_material(sprite_storage.material, "s_diffuse", sampler).unwrap();

        let mut instances = Vec::new();

        for SpriteOptions { transform, tex_coords: (tex_position, tex_dimensions) } in sprites {
            instances.push(Rectangle::default()
                .position([transform.position.0, transform.position.1])
                .dimensions([transform.size.0, transform.size.1])
                .tex_position(tex_position)
                .tex_dimensions(tex_dimensions)
                .depth(transform.depth)
                .color([1.0; 3])
                .opacity(1.0));
        }

        let work = RenderWork {
            vertices: vertices.clone(),
            indices: indices.clone(),
            instances: Some(instances),
            material: sprite_storage.material,
        };

        render_api.submit_subrender(&[work], None).unwrap();
    }
}