use crate::physics::intent::CellIntent;
use crate::physics::module::Module;
use crate::physics::util;
use crate::physics::util::rand_iter_dir;
use crate::world::{CurrCtx, NextCtx};
use macroquad::rand::gen_range;
use serde_json::Value;
use std::collections::HashMap;

pub struct ModuleReactionsBasic {

}

impl ModuleReactionsBasic {
    pub fn new(curr: &CurrCtx<'_>) -> Self {
        Self  {}
    }
}

impl Module for ModuleReactionsBasic {

    fn apply_config(&mut self, config: &HashMap<String, Value>) {}

    fn run(&mut self, curr: &CurrCtx<'_>, next: &mut NextCtx<'_>) {}

    fn gather_intents(&mut self, curr: &CurrCtx<'_>) -> Vec<CellIntent> {

        let mut intents = vec![];

        rand_iter_dir(curr.w, curr.h, |x, y| {

            // Get material of this cell.
            let mat = curr.get_mat_id(x, y);

            // Check neighbors in random order for reactive materials.
            util::try_random_dirs(true, |(dx, dy)| {
                let nx = x as isize + dx;
                let ny = y as isize + dy;

                // Check out of bounds.
                if (!curr.contains(nx, ny)) { return false; }

                // Get material of this neighbor.
                let neigh_mat = curr.get_mat_id(nx as usize, ny as usize);

                // Check if this neighbor is reactive.
                if let Some(react_id) = curr.react_db.get_reaction_by_mats(mat, neigh_mat) {
                    if let Some(react) = curr.react_db.get(react_id) {

                        // Roll dice for rate.
                        if gen_range(0.0, 1.0) > react.rate {
                            return false;
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
                        return true;
                    }
                }
                return false;
            });
        });

        intents
    }
}
