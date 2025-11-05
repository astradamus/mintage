use crate::material::MaterialId;
use crate::physics::module::Module;
use crate::physics::util;
use crate::world::{CurrCtx, NextCtx};
use macroquad::rand::gen_range;
use serde_json::Value;
use std::collections::HashMap;

pub struct ModuleBehaviorSteam {
    mat_id_steam: MaterialId,
    mat_id_air: MaterialId,
    fade_chance: f32,
    changed: Vec<bool>,
}

impl ModuleBehaviorSteam {
    pub fn new(curr: &CurrCtx<'_>) -> Self {
        Self { // TODO Hot reload support.
            mat_id_steam: curr.mat_db.get_id("base:steam").expect("steam material not found"),
            mat_id_air: curr.mat_db.get_id("base:air").expect("air material not found"),
            fade_chance: 0.0,
            changed: vec![false; curr.w * curr.h],
        }
    }
}

impl Module for ModuleBehaviorSteam {

    fn apply_config(&mut self, config: &HashMap<String, Value>) {
        if let Some(Value::Number(n)) = config.get("steam_fade_chance") {
            self.fade_chance =
                (n.as_f64().unwrap() as f32)
                .clamp(0.0, 1.0);
        }
        else {
            panic!("Config missing 'steam_fade_chance'!");
        }
    }

    fn run(&mut self, curr: &CurrCtx<'_>, next: &mut NextCtx<'_>) {

        // Clear changed.
        self.changed.fill(false);

        util::rand_iter_dir(curr.w, curr.h, |x, y| {

            // Check if already changed.
            if (self.changed[y * curr.w + x]) {
                return;
            }

            let a = curr.get_mat_id(x, y);
            if (a == self.mat_id_steam) {

                // Chance to fade.
                let result = gen_range(0.0, 1.0);
                if result < self.fade_chance {
                    next.set_mat_id(x, y, self.mat_id_air);
                    self.changed[y * curr.w + x] = true;
                    return;
                }

                // Check directions in random order.
                util::try_random_dirs(false, |(dx, dy)| {
                    let nx = x as isize + dx;
                    let ny = y as isize + dy;

                    // Check out of bounds.
                    if (!curr.contains(nx, ny)) { return false; }

                    // Check if already changed.
                    if (self.changed[ny as usize * curr.w + nx as usize]) {
                        return false;
                    }

                    let b = curr.get_mat_id(nx as usize, ny as usize);
                    if (b == self.mat_id_air) {
                        next.set_mat_id(x, y, self.mat_id_air);
                        next.set_mat_id(nx as usize, ny as usize, self.mat_id_steam);
                        self.changed[y * curr.w + x] = true;
                        self.changed[ny as usize * curr.w + nx as usize] = true;
                        return true;
                    }
                    false
                });
            }
        });
    }
}
