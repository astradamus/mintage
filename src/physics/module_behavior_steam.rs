use crate::material::MaterialId;
use crate::physics::intent::CellIntent;
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
}

impl ModuleBehaviorSteam {
    pub fn new(curr: &CurrCtx<'_>) -> Self {
        Self { // TODO Hot reload support.
            mat_id_steam: curr.mat_db.get_id("base:steam").expect("steam material not found"),
            mat_id_air: curr.mat_db.get_id("base:air").expect("air material not found"),
            fade_chance: 0.0,
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

    }

    fn gather_intents(&mut self, curr: &CurrCtx<'_>) -> Vec<CellIntent> {

        let mut intents = vec![];

        util::rand_iter_dir(curr.w, curr.h, |x, y| {

            let a = curr.get_mat_id(x, y);
            if (a == self.mat_id_steam) {

                // Chance to fade.
                let result = gen_range(0.0, 1.0);
                if result < self.fade_chance {
                    intents.push(CellIntent::Transform { cell: (x, y), out: self.mat_id_air });
                    return;
                }

                // Check directions in random order.
                util::try_random_dirs(false, |(dx, dy)| {
                    let nx = x as isize + dx;
                    let ny = y as isize + dy;

                    // Check out of bounds.
                    if (!curr.contains(nx, ny)) { return false; }

                    let b = curr.get_mat_id(nx as usize, ny as usize);
                    if (b == self.mat_id_air) {
                        intents.push(CellIntent::Movement { from: (x, y), to: (nx as usize, ny as usize)});
                        return true;
                    }
                    false
                });
            }
        });

        intents
    }
}
