use crate::physics::intent::CellIntent;
use crate::physics::module::{Module, ModuleOutput};
use crate::physics::util::{rand_iter_dir, NEIGHBORS_4};
use crate::world::{CurrCtx, PostRunCtx};
use rand::{Rng, SeedableRng};
use rand_xoshiro::Xoshiro256PlusPlus;
use serde_json::Value;
use std::collections::HashMap;

pub struct ModuleReactionsBasic {
    rng_a: Xoshiro256PlusPlus,
    rng_b: Xoshiro256PlusPlus,
}

impl ModuleReactionsBasic {
    pub fn new(curr: &CurrCtx<'_>, rng_seed: u64) -> Self {
        Self  {
            rng_a: Xoshiro256PlusPlus::seed_from_u64(rng_seed),
            rng_b: Xoshiro256PlusPlus::seed_from_u64(rng_seed ^ 0xBBBBBBBBBBBBBBBB),
        }
    }
}

impl Module for ModuleReactionsBasic {

    fn apply_config(&mut self, config: &HashMap<String, Value>) {}

    fn run(&mut self, curr: &CurrCtx<'_>) -> ModuleOutput {

        let mut intents = vec![];

        rand_iter_dir(&mut self.rng_a, curr.w, curr.h, |x, y| {

            // Get material of this cell.
            let mat = curr.get_mat_id(x, y);

            // Check neighbors for reactive materials.
            // TODO Was doing this in random order, but fixed order is SO MUCH FASTER.
            // TODO Keep an eye on, I think it might be okay as fixed order. Bias probably not noticeable?
            for neighbor in NEIGHBORS_4 {
                let dx = neighbor.0;
                let dy = neighbor.1;
                let nx = x as isize + dx;
                let ny = y as isize + dy;

                // Check out of bounds.
                if (!curr.contains(nx, ny)) { continue; }

                // Get material of this neighbor.
                let neigh_mat = curr.get_mat_id(nx as usize, ny as usize);

                // Check if this neighbor is reactive.
                if let Some(react_id) = curr.react_db.get_reaction_by_mats(mat, neigh_mat) {
                    if let Some(react) = curr.react_db.get(react_id) {

                        // Roll dice for rate.
                        if self.rng_b.random_range(0.0..1.0) > react.rate {
                            continue;
                        }

                        // Reaction found. Sort which cell is a or b.
                        let (ax, ay) = if react.in_a == mat { (x, y) } else { (nx as usize, ny as usize) };
                        let (bx, by) = if react.in_a == mat { (nx as usize, ny as usize) } else { (x, y) };

                        // Register reaction intent.
                        intents.push(CellIntent::Reaction {
                            cell_a: (ax, ay),
                            cell_b: (bx, by),
                            out_a: react.out_a,
                            out_b: react.out_b,
                        });
                        break;
                    }
                }
            }
        });

        ModuleOutput::CellIntents { intents }
    }

    fn post_run(&mut self, post: &PostRunCtx<'_>, changed_cells: &[usize]) {}
}
