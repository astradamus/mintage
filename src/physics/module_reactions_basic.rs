use crate::physics::module::Module;
use crate::physics::util;
use crate::world::{CurrCtx, NextCtx};
use serde_json::Value;
use std::collections::HashMap;

pub struct ModuleReactionsBasic {

}

impl ModuleReactionsBasic {
    pub fn new(curr: &CurrCtx<'_>) -> Self {
        Self { /* TODO Hot reload support.*/ }
    }
}

impl Module for ModuleReactionsBasic {

    fn apply_config(&mut self, config: &HashMap<String, Value>) {}

    fn run(&mut self, curr: &CurrCtx<'_>, next: &mut NextCtx<'_>) {

        util::rand_iter_dir(curr.w, curr.h, |x, y| {
            // Get material of this cell.
            let mat = next.get_mat_id(x, y);

            // Skip this cell if it's already changed material this frame.
            if curr.get_mat_id(x, y) != mat { return; }

            // Check neighbors in random order for reactive materials.
            let moved = util::try_random_dirs(true, |(dx, dy)| {
                let nx = x as isize + dx;
                let ny = y as isize + dy;
                if (!curr.contains(nx, ny)) { return false; }

                // Get material of this neighbor.
                let neigh_mat = next.get_mat_id(nx as usize, ny as usize);

                // Skip this neighbor if it's already changed material this frame.
                if curr.get_mat_id(nx as usize, ny as usize) != neigh_mat { return false; }

                // Check if this neighbor is reactive.
                if let Some(react_id) = curr.react_db.get_reaction_by_mats(mat, neigh_mat) {
                    if let Some(react) = curr.react_db.get(react_id) {

                        // Reaction found. Sort which cell is a or b.
                        let (ax, ay) = if react.in_a == mat { (x, y) } else { (nx as usize, ny as usize) };
                        let (bx, by) = if react.in_a == mat { (nx as usize, ny as usize) } else { (x, y) };

                        // Apply reaction outputs. TODO Rates!
                        next.set_mat_id(ax, ay, react.out_a);
                        next.set_mat_id(bx, by, react.out_b);
                        return true;
                    }
                }
                false
            });
        });
    }
}
