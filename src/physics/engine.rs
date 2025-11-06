use crate::material::{MaterialDb, MaterialId};
use crate::physics::intent::CellIntent;
use crate::physics::module::Module;
use crate::world::{CurrCtx, NextCtx, World};
use rayon::prelude::{IntoParallelRefMutIterator, ParallelIterator};
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::sync::Arc;

pub struct Engine {
    modules: Vec<Box<dyn Module + Send>>,
    config: HashMap<String, Value>,
    mat_id_air: MaterialId,
}

impl Engine {
    pub fn new(mat_db: &Arc<MaterialDb>) -> Self {
        let path = format!("{}/assets/config.ron", env!("CARGO_MANIFEST_DIR"));
        let contents = fs::read_to_string(&path).expect("Missing config: config.ron");
        let cfg: HashMap<String, Value> = ron::de::from_str(&contents).unwrap();

        Self {
            modules: vec![],
            config: cfg,
            mat_id_air: mat_db.get_id("base:air").unwrap(),
        }
    }

    pub fn add<M: Module + 'static>(&mut self, mut m: M) {
        m.apply_config(&self.config);
        self.modules.push(Box::new(m));
    }

    pub fn step(&mut self, world: &mut World) {

        // Copy curr buffer to next buffer.
        world.sync_all();

        // Get world contexts.
        let (curr, mut next) = world.ctx_pair();

        // Gather intents from modules in parallel.
        // Gather order is deterministic within modules.
        // Intents are applied in the same order as they were gathered.
        // Earlier intents apply first, blocking later ones.
        let intents_by_module: Vec<Vec<CellIntent>> = self.modules
            .par_iter_mut()
            .map(|m| m.gather_intents(&curr))
            .collect();

        // Concatenate intents in module order to preserve determinism.
        let all_intents: Vec<CellIntent> = intents_by_module.into_iter().flatten().collect();
        self.apply_intents(&curr, &mut next, all_intents);

        // Commit the frame.
        world.swap_all();
    }

    fn apply_intents(&self, curr: &CurrCtx<'_>, next: &mut NextCtx<'_>, mut intents: Vec<CellIntent>) {
        let mut changed = vec![false; curr.w * curr.h];

        for intent in intents {
            let cells = intent.affected_cells();

            // Check if any involved cell was already changed this frame.
            if cells.iter().any(|(x, y)| changed[y * curr.w + x]) {
                continue; // Skip this intent due to conflict with previous intent.
            }

            // Mark cells as changed.
            for (x, y) in &cells {
                changed[y * curr.w + x] = true;
            }

            // Apply the action.
            match intent {
                CellIntent::Transform { cell, out } => {
                    next.set_mat_id(cell.0, cell.1, out);
                },
                CellIntent::Reaction { cell_a, cell_b, out_a, out_b } => {
                    next.set_mat_id(cell_a.0, cell_a.1, out_a);
                    next.set_mat_id(cell_b.0, cell_b.1, out_b);
                },
                CellIntent::Movement { from, to } => {
                    let mat = curr.get_mat_id(from.0, from.1);
                    next.set_mat_id(from.0, from.1, self.mat_id_air);
                    next.set_mat_id(to.0, to.1, mat);
                },
            }
        }
    }
}
