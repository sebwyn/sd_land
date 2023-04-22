pub trait Component {
    fn name(&self) -> &'static str;
    fn event(&mut self, entity: &mut Entity);
    fn update(&mut self, entity: &mut Entity);
}

pub struct Scene {
    entities: Vec<Entity>,
}

pub struct Entity {
    name: Option<String>,
    components: Vec<Box<dyn Component>>
}

impl Entity {
    fn new() -> Self {
        Self { name: None, components: Vec::new() }
    }

    fn new_named(name: &str) -> Self {
        Self { name: Some(name.to_string()), components: Vec::new() }
    }

    pub fn name(&self) -> Option<String> { self.name.clone() }

    pub fn get_component(&mut self, name: &str) -> Option<&mut Box<dyn Component>> {
        let position = self.components.iter().position(|c| c.name() == name)?;
        self.components.get_mut(position)
    }

    pub fn component(&mut self, component: Box<dyn Component>) {
        self.components.push(component);
    }
}

impl Scene {
    pub fn new() -> Self {
        Self {
            entities: Vec::new()
        }
    }

    pub fn new_entity(&mut self, ) -> usize {
        self.entities.push(Entity::new());
        self.entities.len() - 1
    }

    pub fn get_entity(&mut self, id: usize) -> Option<&mut Entity> {
        self.entities.get_mut(id)
    }

    pub fn push_component(&mut self, id: usize, component: Box<dyn Component>) {
        if let Some(entity) = self.entities.get_mut(id) {
            entity.component(component);
        }
    }

    pub fn remove_component(&mut self, entity: usize, component: u32) {}

    pub fn add_image(&mut self) {

    }
}


struct TestComponent {
    a: i32,
    b: i32
}

impl Component for TestComponent {
    fn name(&self) -> &'static str {
        "TestComponent"
    }

    fn event(&mut self, entity: &mut Entity) {
        
    }

    fn update(&mut self, entity: &mut Entity) {
        self.a += 1;
        self.b += 1;
    }
}


#[test]
fn test_entity_creation() {
    let mut scene = Scene::new();
    let id = scene.new_entity();

    if let Some(entity) = scene.get_entity(id) {
        entity.component(Box::new(TestComponent { a: 0, b: 10 }));
    }
}