use crate::physics::intent::CellIntent;
use crate::physics::module::{Module, ModuleOutput};
use crate::physics::util::{rand_iter_dir};
use crate::world::{CurrCtx, PostRunCtx};
use rand::{SeedableRng};
use rand_xoshiro::Xoshiro256PlusPlus;
use serde_json::Value;
use std::collections::HashMap;

pub struct ModuleTransformsThermal {
    rng: Xoshiro256PlusPlus,
    checkerboard_toggle: bool,
}

impl ModuleTransformsThermal {
    pub fn new(curr: &CurrCtx<'_>, rng_seed: u64) -> Self {
        Self {
            rng: Xoshiro256PlusPlus::seed_from_u64(rng_seed),
            checkerboard_toggle: false,
        }
    }
}

/// Iterate over all cells in a random order, checking for temperature-based material
/// changes (such as melting).
impl Module for ModuleTransformsThermal {

    fn apply_config(&mut self, config: &HashMap<String, Value>) {}

    fn run(&mut self, curr: &CurrCtx<'_>) -> ModuleOutput {

        let mut intents = vec![];

        self.checkerboard_toggle = !self.checkerboard_toggle;

        rand_iter_dir(&mut self.rng, curr.w, curr.h, |x, y| {

            // Checkerboard: False, skip evens. True, skip odds.
            if ((x + y) & 1) == self.checkerboard_toggle as usize {
                return;
            }

            let id = curr.get_mat_id(x, y);
            if let Some(mat) = curr.mat_db.get(id) {

                // Check cold transform.
                if let Some(cold_mat_id) = mat.transform_cold_mat_id {
                    if (curr.get_temp(x, y) < mat.transform_cold_temp) {
                        intents.push(CellIntent::Transform { cell: (x, y), out: cold_mat_id });
                        return;
                    }
                }

                // Check hot transform.
                if let Some(hot_mat_id) = mat.transform_hot_mat_id {
                    if (curr.get_temp(x, y) > mat.transform_hot_temp) {
                        intents.push(CellIntent::Transform { cell: (x, y), out: hot_mat_id });
                        return;
                    }
                }
            }
        });

        ModuleOutput::CellIntents { intents }
    }

    fn post_run(&mut self, post: &PostRunCtx<'_>, changed_cells: &[usize]) {}
}
