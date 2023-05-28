use std::{collections::BTreeMap, sync::atomic::AtomicUsize};
use legion::{Entity, component, system, Query};
use legion::systems::CommandBuffer;
use legion::world::SubWorld;

#[derive(Clone)]
pub struct Element {
    id: usize,
    parent: Option<usize>,
    demands: DemandedLayout,
    layout: Option<LayoutProvider>,
}

static LAYOUT_INDEX: AtomicUsize = AtomicUsize::new(0);


impl Element {
    pub fn new(demands: DemandedLayout) -> Self {    
        let id = LAYOUT_INDEX.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        Self {
            id,
            parent: None,
            demands,
            layout: None,
        }
    }

    pub fn child_layout_provider(mut self, provider: LayoutProvider) -> Self {
        self.layout = Some(provider); self
    }

    pub fn parent(mut self, parent_layout: &Element) -> Self {
        self.parent = Some(parent_layout.id); self
    }

    pub fn add_child(&self, child_layout: &mut Element) {
        child_layout.parent = Some(self.id)
    }

    pub fn add_parent(&mut self, parent_layout: &Element) {
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

#[derive(Default, Clone, Debug)]
pub struct Transform {
    pub size: (f32, f32),
    pub position: (f32, f32),
    pub depth: f32,
    pub visible: bool
}

impl Transform {
    pub fn contains_point(&self, point: (f32, f32)) -> bool {
        self.position.0 < point.0 && point.0 < self.position.0 + self.size.0 &&
        self.position.1 < point.1 && point.1 < self.position.1 + self.size.1
    }
}

#[derive(Clone, Debug)]
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

#[derive(Debug)]
struct LayoutNode<'a> {
    demands: DemandedLayout,
    provider: Option<&'a LayoutProvider>,

    children_indices: Vec<usize>, //indices referencing the associated nodes data-structure

    id: usize, //world id of node
    parent_id: Option<usize>, //world id of parent

    transform: &'a mut Transform,
}

#[system(for_each)]
#[filter(component::<Element>())]
#[filter(!component::<Transform>())]
pub fn insert_transform(entity: &Entity, cmd: &mut CommandBuffer) {
    cmd.add_component(*entity, Transform::default());
}

#[system]
pub fn layout_on_update(world: &mut SubWorld, query: &mut Query<(&Element, &mut Transform)>, #[resource] screen_size: &(f32, f32)) {
    let mut layouts = query.iter_mut(world)
        .map(|(layout, transform)| {
            LayoutNode {
                id: layout.id,
                parent_id: layout.parent,
                children_indices: Vec::new(),

                demands: layout.demands.clone(),
                provider: layout.layout.as_ref(),
                transform
            }
        })
        .collect::<Vec<_>>();

    layouts.sort_by(|a, b| a.id.cmp(&b.id));

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
    let screen_dimensions = *screen_size;
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
            // LayoutProvider::Vertical => vertical_layout(parent_transform, &demanded_layouts),
            LayoutProvider::Custom(layout) => layout(parent_transform, &demanded_layouts),
            LayoutProvider::Vertical => panic!("Vertical layout not supported. Working on it!"),
        };

        for (transform, index) in transforms.into_iter().zip(visible_children) {
            *nodes.get_mut(index).unwrap().transform = transform.clone();

            update_layout_for_node(index, &transform, nodes);
        }
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn test_layout_supports_box_sizing() {

    }
}