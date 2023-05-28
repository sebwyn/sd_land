use std::collections::HashMap;
use legion::{component, IntoQuery, Query, system};
use legion::systems::Builder;
use legion::world::SubWorld;
use simple_error::SimpleError;
use crate::layout::Transform;
use crate::renderer::camera::Camera;
use crate::renderer::pipeline::Pipeline;
use crate::renderer::primitive::{Rectangle, RectangleVertex, Vertex};
use crate::renderer::render_api::{MaterialHandle, RenderApi, RenderWork};
use crate::renderer::shader_types::{Matrix, Sampler, Texture};

pub struct ActiveSceneCamera;

pub struct Sprite {
    pub image_path: String,
    pub tex_origin: (f32, f32),
    pub tex_dimensions: (f32, f32)
}

pub struct SpriteRenderer {
    images: HashMap<String, Texture>,
    material: MaterialHandle,

    default_sampler: Sampler,
}

impl SpriteRenderer {
    pub fn new(render_api: &mut RenderApi) -> Result<Self, SimpleError>{
        let pipeline = Pipeline::load(include_str!("shaders/sprite.wgsl"))?
            .with_vertex::<RectangleVertex>()
            .with_instance::<Rectangle>();

        let pipeline_handle = render_api.create_pipeline(pipeline);

        let material = render_api.create_material(pipeline_handle)?;
        let default_sampler = Sampler::new(render_api.create_sampler());


        Ok(Self {
            images: HashMap::new(),
            material,
            default_sampler
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
    sprite_query: &mut Query<(&Sprite, &Transform)>,
    #[resource] render_api: &mut RenderApi
) {
    let active_camera = <&Camera>::query().filter(component::<ActiveSceneCamera>()).iter(world).next().unwrap();
    let scene_view_proj_matrix = Matrix::from(active_camera.matrix());
    render_api.update_material(sprite_storage.material, "view_proj", scene_view_proj_matrix.clone()).unwrap();

    let mut sprites_by_image = HashMap::new();

    for (sprite, transform) in sprite_query.iter(world) {
        if transform.visible {
            let sprites = sprites_by_image.entry(sprite.image_path.as_str())
                .or_insert(Vec::new());

            sprites.push((sprite, transform));
        }
    }

    let vertices = Rectangle::VERTICES.to_vec();
    let indices = Rectangle::INDICES.to_vec();

    for (image_path, sprites) in sprites_by_image {
        let texture = &*sprite_storage.images.entry(image_path.to_string())
            .or_insert_with(|| {
                //load the image
                Texture::new(render_api.load_texture(image_path).unwrap())
            });

        render_api.update_material(sprite_storage.material, "t_diffuse", texture.clone()).unwrap();
        render_api.update_material(sprite_storage.material, "s_diffuse", sprite_storage.default_sampler.clone()).unwrap();

        let mut instances = Vec::new();

        //now go ahead and render all the sprites
        for (sprite, transform) in sprites {
            //create a rectangle
            instances.push(Rectangle::default()
                .position([transform.position.0, transform.position.1])
                .dimensions([transform.size.0, transform.size.1])
                .tex_position([sprite.tex_origin.0, sprite.tex_origin.1])
                .tex_dimensions([sprite.tex_dimensions.0, sprite.tex_dimensions.1])
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