use serde_json::Value;
use std::collections::HashMap;
use macroquad::rand::gen_range;
use crate::material::MaterialId;
use crate::world::{ReadCtx, World, WriteCtx};

const NEIGHBORS_8: [(isize, isize); 8] = [
    (-1, -1), (0, -1), (1, -1),
    (-1,  0),          (1,  0),
    (-1,  1), (0,  1), (1,  1),
];
const NEIGHBORS_4: [(isize, isize); 4] = [
              (0, -1),
    (-1,  0),          (1,  0),
              (0,  1),
];

pub fn try_random_dirs<F>(use_4: bool, mut try_dir: F) -> bool
where
    F: FnMut((isize, isize)) -> bool,
{
    let mut rem = [0, 1, 2, 3, 4, 5, 6, 7];
    let mut len = if (use_4) { 4 } else { 8 };

    while len > 0 {
        let r = gen_range(0, len);
        let i = rem[r];

        len -= 1;
        rem.swap(r, len);

        let n = if (use_4) { NEIGHBORS_4[i] } else { NEIGHBORS_8[i] };
        if try_dir(n) {
            return true;
        }
    }

    false
}

pub trait PhysicsModule {
    fn name(&self) -> &'static str;
    fn apply_config(&mut self, config: &HashMap<String, Value>);
    fn run(&mut self, read: &ReadCtx<'_>, write: &mut WriteCtx<'_>);
}

pub struct PhysicsEngine {
    modules: Vec<Box<dyn PhysicsModule>>,
    config: HashMap<String, Value>
}

impl PhysicsEngine {
    pub fn new() -> Self {
        let cfg: HashMap<String, Value> =
            ron::de::from_str(include_str!("../assets/config.ron")).unwrap();

        Self {
            modules: vec![],
            config: cfg,
        }
    }

    pub fn add<M: PhysicsModule + 'static>(&mut self, mut m: M) {
        m.apply_config(&self.config);
        self.modules.push(Box::new(m));
    }

    pub fn step(&mut self, world: &mut World) {
        // Copy read buffer to write buffer
        world.sync_all();

        // Run all physics modules in order
        for m in self.modules.iter_mut() {
            let (read, mut write) = world.ctx_pair();
            m.run(&read, &mut write);
        }

        // Commit the frame
        world.swap_all();
    }
}

pub struct SteamBehavior {
    mat_id_steam: MaterialId,
    mat_id_air: MaterialId,
    fade_chance: f32,
}
impl SteamBehavior {
    pub fn new(read: &ReadCtx<'_>) -> Self {
        Self { // TODO Hot reload support.
            mat_id_steam: read.materials.get_id("base:steam").expect("steam material not found"),
            mat_id_air: read.materials.get_id("base:air").expect("air material not found"),
            fade_chance: 0.0,
        }
    }
}
impl PhysicsModule for SteamBehavior {

    fn name(&self) -> &'static str {"SteamBehavior"}

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

    fn run(&mut self, read: &ReadCtx<'_>, write: &mut WriteCtx<'_>) {

        for y in 0..read.h {
            for x in 0..read.w {

                // Must check write to ensure we see changes made by other modules.
                let a = write.cell_mut(x,y).mat_id;
                if (a == self.mat_id_steam) {

                    // Chance to fade.
                    let result = gen_range(0.0, 1.0);
                    if result < self.fade_chance {
                        write.cell_mut(x, y).mat_id = self.mat_id_air;
                        continue;
                    }

                    // Check directions in random order.
                    let moved = try_random_dirs(false, |(dx, dy)| {
                        let nx = x as isize + dx;
                        let ny = y as isize + dy;
                        if (!read.contains(nx, ny)) { return false; }

                        let b = write.cell_mut(nx as usize, ny as usize).mat_id;
                        if (b == self.mat_id_air) {
                            write.cell_mut(x, y).mat_id = self.mat_id_air;
                            write.cell_mut(nx as usize, ny as usize).mat_id = self.mat_id_steam;
                            return true;
                        }
                        false
                    });
                }
            }
        }
    }
}

pub struct BasicReactions {

}

impl BasicReactions {
    pub fn new(read: &ReadCtx<'_>) -> Self {
        Self { /* TODO Hot reload support.*/ }
    }

    fn do_loop(&self, read: &ReadCtx<'_>, write: &mut WriteCtx<'_>, x: usize, y: usize) {

        // Get material of this cell.
        let mat = write.cell_mut(x, y).mat_id;

        // Skip this cell if it's already changed material this frame.
        if read.get_last_frame_cell(x, y).mat_id != mat { return; }

        // Check neighbors in random order for reactive materials.
        let moved = try_random_dirs(true, |(dx, dy)| {
            let nx = x as isize + dx;
            let ny = y as isize + dy;
            if (!read.contains(nx, ny)) { return false; }

            // Get material of this neighbor.
            let neigh_mat = write.cell_mut(nx as usize, ny as usize).mat_id;

            // Skip this neighbor if it's already changed material this frame.
            if read.get_last_frame_cell(nx as usize, ny as usize).mat_id != neigh_mat { return false; }

            // Check if this neighbor is reactive.
            if let Some(react_id) = read.reactions.get_reaction_by_mats(mat, neigh_mat) {
                if let Some(react) = read.reactions.get(react_id) {

                    // Reaction found. Sort which cell is a or b.
                    let (ax, ay) = if react.in_a == mat { (x, y) } else { (nx as usize, ny as usize) };
                    let (bx, by) = if react.in_a == mat { (nx as usize, ny as usize) } else { (x, y) };

                    // Apply reaction outputs. TODO Rates!
                    write.cell_mut(ax, ay).mat_id = react.out_a;
                    write.cell_mut(bx, by).mat_id = react.out_b;
                    return true;
                }
            }
            false
        });
    }

}

impl PhysicsModule for BasicReactions {

    fn name(&self) -> &'static str {"BasicReactions"}
    fn apply_config(&mut self, config: &HashMap<String, Value>) {}
    fn run(&mut self, read: &ReadCtx<'_>, write: &mut WriteCtx<'_>) {

        let r = gen_range(0, 4) as usize;

        // Do loops in different directions to prevent bias, chosen randomly each frame.
        if (r == 0) {
            for y in 0..read.h {
                for x in 0..read.w {
                    self.do_loop(read, write, x, y);
                }
            }
        }
        else if (r == 1) {
            for y in (0..read.h).rev() {
                for x in (0..read.w) {
                    self.do_loop(read, write, x, y);
                }
            }
        }
        else if (r == 2) {
            for y in (0..read.h).rev() {
                for x in (0..read.w).rev() {
                    self.do_loop(read, write, x, y);
                }
            }
        }
        else if (r == 3) {
            for y in (0..read.h) {
                for x in (0..read.w).rev() {
                    self.do_loop(read, write, x, y);
                }
            }
        }
    }
}