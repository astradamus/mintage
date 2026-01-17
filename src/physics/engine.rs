use crate::physics::intent::CellIntent;
use crate::physics::module::{Module, ModuleOutput};
use crate::world::{CurrCtx, NextCtx, World};
use rayon::prelude::{IntoParallelRefMutIterator, ParallelIterator};
use serde_json::Value;
use std::collections::HashMap;
use std::fs;

pub struct Engine {
    modules: Vec<Box<dyn Module + Send>>,
    config: HashMap<String, Value>,
    changed_dense: Vec<bool>,
    changed_sparse: Vec<usize>,
}

impl Engine {
    pub fn new(config: HashMap<String, Value>, world_w: usize, world_h: usize) -> Self {
        Self {
            modules: vec![],
            config,
            changed_dense: vec![false; world_w * world_h],
            changed_sparse: vec![],
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
        let outputs: Vec<ModuleOutput> = self.modules
            .par_iter_mut()
            .map(|m| m.run(&curr))
            .collect();

        // Apply outputs in module order to preserve determinism.
        for out in outputs {
            match out {
                ModuleOutput::CellIntents { intents } => {
                    self.apply_intents(&curr, &mut next, &intents);
                }
                ModuleOutput::DeltaTemp { delta_temp } => {
                    self.apply_delta_temp(&curr, &mut next, &delta_temp);
                }
            }
        }

        // Get post-run context.
        let post = world.ctx_post_run();

        // Let modules run post-step updates. Some modules want to pre-compute values for speed.
        self.modules
            .par_iter_mut()
            .for_each(|m| m.post_run(&post, self.changed_sparse.as_slice()));

        // Reset changed flags for next frame.
        for &i in &self.changed_sparse {
            self.changed_dense[i] = false;
        }
        self.changed_sparse.clear();

        // Commit the frame.
        world.swap_all();
    }

    fn apply_intents(&mut self, curr: &CurrCtx<'_>, next: &mut NextCtx<'_>, intents: &[CellIntent]) {

        for intent in intents {
            let cells = intent.affected_cells();

            // Check if any involved cell was already changed this frame.
            if cells.iter().any(|(x, y)| self.changed_dense[y * curr.w + x]) {
                continue; // Skip this intent due to conflict with previous intent.
            }

            // Mark cells as changed.
            for (x, y) in &cells {
                let i = y * curr.w + x;
                self.changed_dense[i] = true;
                self.changed_sparse.push(i);
            }

            // Apply the action.
            match intent {
                &CellIntent::Transform { cell, out } => {
                    next.set_mat_id(cell.0, cell.1, out);
                },
                &CellIntent::Reaction { cell_a, cell_b, out_a, out_b } => {
                    next.set_mat_id(cell_a.0, cell_a.1, out_a);
                    next.set_mat_id(cell_b.0, cell_b.1, out_b);
                },
                &CellIntent::MoveSwap { from, to } => {
                    let mat_from = curr.get_mat_id(from.0, from.1);
                    let mat_to = curr.get_mat_id(to.0, to.1);
                    next.set_mat_id(from.0, from.1, mat_to);
                    next.set_mat_id(to.0, to.1, mat_from);

                    // Peek future temp, because we want to make sure temp changes affect this particle.
                    // Note that we do not peek future mat id. That's because mat id changes
                    // prevent move swap intents from being applied in the same tick.
                    let temp_from = next.peek_future_temp(from.0, from.1);
                    let temp_to = next.peek_future_temp(to.0, to.1);
                    next.set_temp(from.0, from.1, temp_to);
                    next.set_temp(to.0, to.1, temp_from);
                },
            }
        }
    }

    fn apply_delta_temp(&self, curr: &CurrCtx<'_>, next: &mut NextCtx<'_>, delta_temp: &[f32]) {
        for i in 0..(curr.w * curr.h) {
            next.add_temp_i(i, delta_temp[i]);
        }
    }
}
