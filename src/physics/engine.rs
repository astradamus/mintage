use crate::physics::module::Module;
use crate::world::World;
use serde_json::Value;
use std::collections::HashMap;
use std::fs;

pub struct Engine {
    modules: Vec<Box<dyn Module>>,
    config: HashMap<String, Value>
}

impl Engine {
    pub fn new() -> Self {
        let path = format!("{}/assets/config.ron", env!("CARGO_MANIFEST_DIR"));
        let contents = fs::read_to_string(&path).expect("Missing config: config.ron");
        let cfg: HashMap<String, Value> = ron::de::from_str(&contents).unwrap();

        Self {
            modules: vec![],
            config: cfg,
        }
    }

    pub fn add<M: Module + 'static>(&mut self, mut m: M) {
        m.apply_config(&self.config);
        self.modules.push(Box::new(m));
    }

    pub fn step(&mut self, world: &mut World) {
        // Copy curr buffer to next buffer
        world.sync_all();

        // Run all physics modules in order
        for m in self.modules.iter_mut() {
            let (curr, mut next) = world.ctx_pair();
            m.run(&curr, &mut next);
        }

        // Commit the frame
        world.swap_all();
    }
}