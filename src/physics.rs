use macroquad::prelude::rand;
use macroquad::rand::{ChooseRandom};
use crate::material::MaterialId;
use crate::world::{ReadCtx, World, WriteCtx};

const NEIGHBORS_8: [(isize, isize); 8] = [
    (-1, -1), (0, -1), (1, -1),
    (-1,  0),          (1,  0),
    (-1,  1), (0,  1), (1,  1),
];

pub trait PhysicsModule {
    fn name(&self) -> &'static str;
    fn run(&mut self, read: &ReadCtx<'_>, write: &mut WriteCtx<'_>);
}

pub struct PhysicsEngine {
    modules: Vec<Box<dyn PhysicsModule>>,
}

impl PhysicsEngine {
    pub fn new() -> Self { Self { modules: vec![] } }

    pub fn add<M: PhysicsModule + 'static>(&mut self, m: M) {
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

pub(crate) struct ReactionWaterLava {
    mat_id_water: MaterialId,
    mat_id_lava: MaterialId,
    mat_id_stone: MaterialId,
    mat_id_steam: MaterialId,
}
impl ReactionWaterLava {
    pub fn new(read: &ReadCtx<'_>) -> Self {
        Self { // TODO Hot reload support.
            mat_id_water: read.materials.get_id("base:water").expect("water material not found"),
            mat_id_lava: read.materials.get_id("base:lava").expect("lava material not found"),
            mat_id_stone: read.materials.get_id("base:stone").expect("stone material not found"),
            mat_id_steam: read.materials.get_id("base:steam").expect("steam material not found"),
        }
    }
}
impl PhysicsModule for ReactionWaterLava {

    fn name(&self) -> &'static str {"ReactionWaterLava"}
    fn run(&mut self, read: &ReadCtx<'_>, write: &mut WriteCtx<'_>) {

        for y in 0..read.h {
            for x in 0..read.w {
                let a = read.cell(x,y).mat_id;
                if (a == self.mat_id_lava) {

                    // Check directions in random order. TODO Seed determinism.
                    let mut dirs = NEIGHBORS_8;
                    dirs.shuffle();
                    for (dx,dy) in dirs {
                        let nx = x as isize + dx;
                        let ny = y as isize + dy;
                        if (!read.contains(nx, ny)) { continue; }

                        let b = read.cell(nx as usize, ny as usize).mat_id;
                        if (b == self.mat_id_water) {
                            write.cell_mut(x, y).mat_id = self.mat_id_stone;
                            write.cell_mut(nx as usize, ny as usize).mat_id = self.mat_id_steam;
                            // TODO Hmm, one water can convert two lava to steam because of how we check CUR instead of NEXT...
                            // TODO Hmm, one water can convert two lava to steam because of how we check CUR instead of NEXT...
                        }
                    }
                }
            }
        }
    }
}


pub(crate) struct SteamBehavior {
    mat_id_steam: MaterialId,
    mat_id_air: MaterialId,
}
impl SteamBehavior {
    pub fn new(read: &ReadCtx<'_>) -> Self {
        Self { // TODO Hot reload support.
            mat_id_steam: read.materials.get_id("base:steam").expect("steam material not found"),
            mat_id_air: read.materials.get_id("base:air").expect("air material not found"),
        }
    }
}
impl PhysicsModule for SteamBehavior {

    fn name(&self) -> &'static str {"SteamBehavior"}
    fn run(&mut self, read: &ReadCtx<'_>, write: &mut WriteCtx<'_>) {

        for y in 0..read.h {
            for x in 0..read.w {
                let a = read.cell(x,y).mat_id;
                if (a == self.mat_id_steam) {

                    // Chance to dissipate.
                    let result = rand::gen_range(0.0, 1.0);
                    if result < 0.2 {
                        write.cell_mut(x, y).mat_id = self.mat_id_air;
                        continue;
                    }

                    // Check directions in random order. TODO Seed determinism.
                    let mut dirs = NEIGHBORS_8;
                    dirs.shuffle();
                    for (dx,dy) in dirs {
                        let nx = x as isize + dx;
                        let ny = y as isize + dy;
                        if (!read.contains(nx, ny)) { continue; }

                        let b = read.cell(nx as usize, ny as usize).mat_id;
                        if (b == self.mat_id_air) {
                            write.cell_mut(x, y).mat_id = self.mat_id_air;
                            write.cell_mut(nx as usize, ny as usize).mat_id = self.mat_id_steam;
                            break;
                        }
                    }
                }
            }
        }
    }
}