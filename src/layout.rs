use std::{collections::BTreeMap, sync::atomic::AtomicUsize};
use crate::system::Systems;
use legion::{IntoQuery, Entity, component};

pub struct Layout {
    id: usize,
    parent: Option<usize>,
    demands: DemandedLayout,
    provider: Option<LayoutProvider>
}

static LAYOUT_INDEX: AtomicUsize = AtomicUsize::new(0);

impl Layout {
    pub fn new(demands: DemandedLayout) -> Self {    
        let id = LAYOUT_INDEX.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        Self {
            id,
            parent: None,
            demands,
            provider: None,
        }
    }

    pub fn child_layout_provider(mut self, provider: LayoutProvider) -> Self {
        self.provider = Some(provider); self
    }

    pub fn parent(mut self, parent_layout: &Layout) -> Self {
        self.parent = Some(parent_layout.id); self
    }

    pub fn add_child(&self, child_layout: &mut Layout) {
        child_layout.parent = Some(self.id)
    }

    pub fn add_parent(&mut self, parent_layout: &Layout) {
        self.parent = Some(parent_layout.id);
    }
}

#[derive(Clone, Debug)]
pub enum DemandValue {
    Percent(f32),
    Absolute(f32)
}

#[derive(Default, Clone, Debug)]
pub struct DemandedLayout {
    //layout demands
    pub size: Option<[DemandValue; 2]>,
    pub position: Option<[DemandValue; 2]>,

    pub horizontal_index: Option<u32>,
    pub vertical_index: Option<u32>,
    pub depth: Option<f32>,

    pub parent_anchor: Option<[Anchor; 2]>,
    pub child_anchor: Option<[Anchor; 2]>,    

    pub visible: bool
}

#[derive(Default, Clone)]
pub struct Transform {
    pub size: (f32, f32),
    pub position: (f32, f32),
    pub depth: f32,
    pub visible: bool
}

pub enum LayoutProvider {
    Relative,
    Vertical,
    Custom(fn(parent_layout: &Transform, &[&DemandedLayout]) -> Vec<Transform>)
}

#[derive(Clone, Debug)]
pub enum Anchor {
    Min,
    Max,
    Center,
}

fn point_for_anchor(layout: &Transform, anchor: &[Anchor; 2]) -> (f32, f32) {
    let x = 
        match anchor[0] {
            Anchor::Min => layout.position.0,
            Anchor::Max => layout.position.0 + layout.size.0,
            Anchor::Center => layout.position.0 + layout.size.0 / 2.0,
        };

    let y = 
        match anchor[1] {
            Anchor::Min => layout.position.1,
            Anchor::Max => layout.position.1 + layout.size.1,
            Anchor::Center => layout.position.1 + layout.size.1 / 2.0,
        };

    (x, y)
}

//be extremely kind in respecting childrens demands, but also slow in that the layout will make everything visible
fn relative_layout(parent_layout: &Transform, demanded_layouts: &[&DemandedLayout]) -> Vec<Transform> {
    let parent_size = parent_layout.size;
    let parent_position = parent_layout.position;
    
    let mut provided_transforms = Vec::new();

    for demands in demanded_layouts {
        let size = demands.size.as_ref().map(|[width_demand, height_demand]| {
            let width = match width_demand {
                DemandValue::Percent(v) => parent_size.0 * v,
                DemandValue::Absolute(v) => *v,
            };
            let height = match height_demand {
                DemandValue::Percent(v) => parent_size.1 * v,
                DemandValue::Absolute(v) => *v,
            };

            (width, height)
        }).unwrap_or(parent_size);


        let anchor_point = point_for_anchor(parent_layout, demands.parent_anchor.as_ref().unwrap_or(&[Anchor::Min, Anchor::Min]));
        
        let position = demands.position.as_ref().map(|[x_demand, y_demand]| {
            let x = match x_demand {
                DemandValue::Percent(v) => anchor_point.0 + parent_size.0 * v,
                DemandValue::Absolute(v) => anchor_point.0 + *v,
            };
            let y = match y_demand {
                DemandValue::Percent(v) => anchor_point.1 + parent_size.1 * v,
                DemandValue::Absolute(v) => anchor_point.1 + *v,
            };

            (x, y)
        }).unwrap_or(anchor_point);


        let mut transform = Transform {
            position,
            size,
            depth: demands.depth.unwrap_or(0.5),
            visible: demands.visible
        };

        let child_anchor_point = point_for_anchor(&transform, demands.child_anchor.as_ref().unwrap_or(&[Anchor::Min, Anchor::Min]));
        
        let delta = (child_anchor_point.0 - transform.position.0, child_anchor_point.1 - transform.position.1);
        transform.position.0 -= delta.0;
        transform.position.1 -= delta .1;

        provided_transforms.push(transform);
    }
    
    provided_transforms
}

//disregards any vertical positioning
fn vertical_layout(parent_layout: &Transform, demanded_layouts: &[&DemandedLayout]) -> Vec<Transform> {
    let mut enumerated_layouts = demanded_layouts
        .iter()
        .enumerate()
        .filter_map(|(i, layout)| layout.vertical_index.map(|index| (i, index, layout))
        )
        .collect::<Vec<_>>();
            
    enumerated_layouts.sort_by(|(_, vi1, _), (_, vi2, _)| (*vi1).cmp(vi2));

    let mut current_y = parent_layout.position.1 + parent_layout.size.1;

    let mut transforms = vec![Transform::default(); demanded_layouts.len()];

    for (child_index, _, layout) in enumerated_layouts {
        let (width, height) = layout.size.as_ref().map(|size| {
            (
                match size[0] {
                    DemandValue::Percent(v) => parent_layout.size.0 * v,
                    DemandValue::Absolute(v) => v,
                },
                match size[1] {
                    DemandValue::Percent(v) => parent_layout.size.1 * v,
                    DemandValue::Absolute(v) => v,
                }
            )
        }).unwrap_or((0f32, 0f32));

        let x_position = layout.position.as_ref().map(|p| 
            match p[0] {
                DemandValue::Percent(v) => parent_layout.position.0 + parent_layout.size.0 * v,
                DemandValue::Absolute(v) => parent_layout.position.0 + v,
            }
        ).unwrap_or(parent_layout.position.0);

        transforms[child_index] = Transform { 
            size: (width, height), 
            position: (x_position, current_y),
            depth: layout.depth.unwrap_or(parent_layout.depth), 
            visible: layout.visible
        };

        current_y -= height;

        if current_y < parent_layout.position.0 {
            break
        }
    }

    transforms
}

struct LayoutNode<'a> {
    demands: DemandedLayout,
    provider: Option<&'a LayoutProvider>,

    children_indices: Vec<usize>, //indices referencing the associated nodes datastructure

    id: usize, //world id of node
    parent_id: Option<usize>, //world id of parent

    transform: &'a mut Transform,
}

pub fn layout_on_update(world: &mut legion::World, systems: &Systems) {
    //add transforms to any layouts that don't have transforms
    let layouts_without_transforms = <Entity>::query()
        .filter(component::<Layout>() & !component::<Transform>())
        .iter(world)
        .cloned()
        .collect::<Vec<_>>();

    for entity in layouts_without_transforms {
        world.entry(entity).unwrap().add_component(Transform::default());
    }
    
    let layouts = <(&Layout, &mut Transform)>::query()
        .iter_mut(world)
        .map(|(layout, transform)| {
            LayoutNode {
                id: layout.id,
                parent_id: layout.parent,
                children_indices: Vec::new(),

                demands: layout.demands.clone(),
                provider: layout.provider.as_ref(),
                transform
            }
        })
        .collect::<Vec<_>>();

    let mut nodes = Vec::new();

    let mut root_indices = Vec::new();

    let mut parent_id_to_index = BTreeMap::new();
    
    //move the nodes with no parent into the nodes vector
    let mut remaining_layouts = layouts.into_iter().filter_map(|node| {
        if node.parent_id.is_none() {
            root_indices.push(nodes.len());
            parent_id_to_index.insert(node.id, nodes.len());
            nodes.push(node);
            None
        } else {
            Some(node)
        }
    }).collect::<Vec<_>>();

    while ! remaining_layouts.is_empty() {
        let mut child_id_to_index = BTreeMap::new();

        remaining_layouts = remaining_layouts.into_iter().filter_map(|node| {
            if let Some(parent_index) = parent_id_to_index.get(node.parent_id.as_ref().unwrap()) {
                let node_index = nodes.len();
                
                child_id_to_index.insert(node.id, node_index);
                nodes.get_mut(*parent_index).unwrap().children_indices.push(node_index);

                nodes.push(node);
                None
            } else {
                Some(node)
            }
        }).collect();

        parent_id_to_index = child_id_to_index;
    }

    //now go and update all the transform based on the screen size
    let screen_dimensions = systems.screen_size();
    let screen_origin = (0f32, 0f32);

    let screen_layout = Transform {
        size: screen_dimensions,
        position: screen_origin,
        depth: 0.0,
        visible: true,
    };
    
    //gather the demands
    let demanded_layouts = root_indices.iter().map(|i| &nodes[*i].demands).collect::<Vec<_>>();
    let transforms = relative_layout(&screen_layout, &demanded_layouts);

    for (transform, node_index) in transforms.into_iter().zip(root_indices) {
        *nodes.get_mut(node_index).unwrap().transform = transform.clone();

        //update the child layouts
        update_layout_for_node(node_index, &transform, &mut nodes)
    }

}

fn update_layout_for_node(node_index: usize, parent_transform: &Transform, nodes: &mut [LayoutNode]) {
    //gather the demands
    if let Some(provider) = nodes[node_index].provider {
        let visible_children = nodes[node_index].children_indices
            .iter()
            .filter(|i| nodes[**i].demands.visible)
            .cloned()
            .collect::<Vec<_>>();

        let demanded_layouts = visible_children.iter()
            .map(|i| &nodes[*i].demands)
            .collect::<Vec<_>>();

        let transforms = match provider {
            LayoutProvider::Relative => relative_layout(parent_transform, &demanded_layouts),
            LayoutProvider::Vertical => vertical_layout(parent_transform, &demanded_layouts),
            LayoutProvider::Custom(layout) => layout(parent_transform, &demanded_layouts),
        };

        for (transform, index) in transforms.into_iter().zip(visible_children) {
            *nodes.get_mut(index).unwrap().transform = transform.clone();

            update_layout_for_node(index, &transform, nodes);
        }
    }
}

#[cfg(test)]
pub(crate) mod layout_test {
    use legion::{World, EntityStore};
    use winit::dpi::PhysicalSize;

    use super::*;

    #[test]
    fn relative_layout_uses_screen_percentages() {
        let mut systems = Systems::new(PhysicalSize { width: 800, height: 600 });
        systems.register_update_system(layout_on_update);

        let mut world = World::default();

        let entity1_size = [DemandValue::Absolute(15f32), DemandValue::Absolute(25f32)];
        let entity1_position = [DemandValue::Absolute(40f32), DemandValue::Absolute(55f32)];

        let entity1_expected_size = (15f32, 25f32);
        let entity1_expected_position = (40f32, 55f32);

        let entity2_size = [DemandValue::Percent(0.5), DemandValue::Percent(0.5)];
        let entity2_position = [DemandValue::Percent(0.25f32), DemandValue::Percent(0.25f32)];

        let entity2_expected_size = (400f32, 300f32);
        let entity2_expected_position = (200f32, 150f32);

        let entity1_handle = world.push((
            Transform::default(), 
            Layout::new(DemandedLayout { 
                size: Some(entity1_size), 
                position: Some(entity1_position),
                visible: true,
                ..Default::default() 
            }),
        ));

        let entity2_handle = world.push((
            Transform::default(), 
            Layout::new(DemandedLayout { 
                size: Some(entity2_size), 
                position: Some(entity2_position),
                visible: true,
                ..Default::default() 
            }),
        ));
    
        systems.update(&mut world);
        let entity1 = world.entry_ref(entity1_handle).unwrap();
        let transform1 = entity1.get_component::<Transform>().unwrap();
        assert_eq!(entity1_expected_size, transform1.size);
        assert_eq!(entity1_expected_position, transform1.position);

        let entity2 = world.entry_ref(entity2_handle).unwrap();

        let transform2 = entity2.get_component::<Transform>().unwrap();
        assert_eq!(entity2_expected_size, transform2.size);
        assert_eq!(entity2_expected_position, transform2.position);

    }

    #[test]
    fn hierarchies_are_layed_out() {
        let mut systems = Systems::new(PhysicalSize { width: 800, height: 600 });
        systems.register_update_system(layout_on_update);

        let mut world = World::default();

        let entity1_size = [DemandValue::Absolute(15f32), DemandValue::Absolute(25f32)];
        let entity1_position = [DemandValue::Absolute(40f32), DemandValue::Absolute(55f32)];

        let mut layout1 = Layout::new(DemandedLayout { 
            size: Some(entity1_size), 
            position: Some(entity1_position),
            visible: true,
            ..Default::default() 
        });

        let entity2_size = [DemandValue::Percent(0.5), DemandValue::Percent(0.5)];
        let entity2_position = [DemandValue::Percent(0.25f32), DemandValue::Percent(0.25f32)];

        let layout2 = Layout::new(DemandedLayout { 
            size: Some(entity2_size), 
            position: Some(entity2_position), 
            visible: true,
            ..Default::default() 
        }).child_layout_provider(LayoutProvider::Relative);
        
        layout1.add_parent(&layout2);

        let entity1_expected_size = (15f32, 25f32);
        let entity1_expected_position = (200f32 + 40f32, 150f32 + 55f32);

        let entity2_expected_size = (400f32, 300f32);
        let entity2_expected_position = (200f32, 150f32);

        let entities = world.extend([
            (
                Transform::default(), 
                layout1,
            ),
            (
                Transform::default(), 
                layout2,
            ),        
        ]).to_vec();
    
        systems.update(&mut world);
        let entity1 = world.entry_ref(entities[0]).unwrap();
        let transform1 = entity1.get_component::<Transform>().unwrap();
        assert_eq!(entity1_expected_size, transform1.size);
        assert_eq!(entity1_expected_position, transform1.position);

        let entity2 = world.entry_ref(entities[1]).unwrap();

        let transform2 = entity2.get_component::<Transform>().unwrap();
        assert_eq!(entity2_expected_size, transform2.size);
        assert_eq!(entity2_expected_position, transform2.position);

    }

    #[test]
    fn vertical_layouts() {
        let mut systems = Systems::new(PhysicalSize { width: 800, height: 600 });
        systems.register_update_system(layout_on_update);

        let mut world = World::default();

        //create a vertical layout that is wide
        let vertical_layout = Layout::new(DemandedLayout { 
            size: Some([DemandValue::Percent(0.6), DemandValue::Percent(1.0f32)]), 
            position: Some([DemandValue::Percent(0.2), DemandValue::Absolute(0f32)]),
            visible: true ,
            ..Default::default()
        }).child_layout_provider(LayoutProvider::Vertical);

        let child1_layout = Layout::new(DemandedLayout { 
            size: Some([DemandValue::Percent(0.8), DemandValue::Percent(0.5f32)]),
            vertical_index: Some(0),
            visible: true,
            ..Default::default()
        }).parent(&vertical_layout);

        let child2_layout = Layout::new(DemandedLayout { 
            size: Some([DemandValue::Percent(0.8), DemandValue::Absolute(150f32)]),
            position: Some([DemandValue::Percent(0.2), DemandValue::Absolute(0f32)]),
            vertical_index: Some(1),
            visible: true,
            ..Default::default()
        }).parent(&vertical_layout);

        let entities = world.extend([
            (Transform::default(), vertical_layout), (Transform::default(), child2_layout), (Transform::default(), child1_layout)
        ]).to_vec();

        systems.update(&mut world);

        let transforms = entities.into_iter()
            .map(|e| 
                world.entry_ref(e)
                    .unwrap()
                    .get_component::<Transform>()
                    .unwrap()
                    .clone()
            )
            .collect::<Vec<_>>();

        assert_eq!(3, transforms.len());
        assert_eq!(480f32, transforms[0].size.0.round());
        assert_eq!(600f32, transforms[0].size.1.round());

        assert_eq!((160f32, 0f32), transforms[0].position);
        
        assert_eq!(384f32, transforms[1].size.0.round());
        assert_eq!(150f32, transforms[1].size.1.round());
        assert_eq!((256f32, 300f32), transforms[1].position);

        assert_eq!(384f32, transforms[2].size.0.round());
        assert_eq!(300f32, transforms[2].size.1.round());
        assert_eq!((160f32, 600f32), transforms[2].position);
        
    }
}   
