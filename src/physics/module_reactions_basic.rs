use crate::physics::module::Module;
use crate::physics::util;
use crate::world::{CurrCtx, NextCtx};
use serde_json::Value;
use std::collections::HashMap;

pub struct ModuleReactionsBasic {
    changed: Vec<bool>,
}

impl ModuleReactionsBasic {
    pub fn new(curr: &CurrCtx<'_>) -> Self {
        Self  {
            changed: vec![false; curr.w * curr.h],
        }
    }
}

impl Module for ModuleReactionsBasic {

    fn apply_config(&mut self, config: &HashMap<String, Value>) {}

    fn run(&mut self, curr: &CurrCtx<'_>, next: &mut NextCtx<'_>) {

        // Clear changed.
        self.changed.fill(false);

        util::rand_iter_dir(curr.w, curr.h, |x, y| {

            // Check if already changed.
            if (self.changed[y * curr.w + x]) {
                return;
            }

            // Get material of this cell.
            let mat = curr.get_mat_id(x, y);

            // Check neighbors in random order for reactive materials.
            util::try_random_dirs(true, |(dx, dy)| {
                let nx = x as isize + dx;
                let ny = y as isize + dy;

                // Check out of bounds.
                if (!curr.contains(nx, ny)) { return false; }

                // Check if already changed.
                if (self.changed[ny as usize * curr.w + nx as usize]) {
                    return false;
                }

                // Get material of this neighbor.
                let neigh_mat = curr.get_mat_id(nx as usize, ny as usize);

                // Check if this neighbor is reactive.
                if let Some(react_id) = curr.react_db.get_reaction_by_mats(mat, neigh_mat) {
                    if let Some(react) = curr.react_db.get(react_id) {

                        // Reaction found. Sort which cell is a or b.
                        let (ax, ay) = if react.in_a == mat { (x, y) } else { (nx as usize, ny as usize) };
                        let (bx, by) = if react.in_a == mat { (nx as usize, ny as usize) } else { (x, y) };

                        // Apply reaction outputs. TODO Rates!
                        next.set_mat_id(ax, ay, react.out_a);
                        next.set_mat_id(bx, by, react.out_b);
                        self.changed[y * curr.w + x] = true;
                        self.changed[ny as usize * curr.w + nx as usize] = true;
                        return true;
                    }
                }
                false
            });
        });
    }
}
