use crate::material::{MaterialDb, MaterialId};
use crate::physics::engine::Engine;
use crate::physics::module_behavior_steam::ModuleBehaviorSteam;
use crate::physics::module_reactions_basic::ModuleReactionsBasic;
use crate::reaction::ReactionDb;
use crate::world::World;
use arc_swap::ArcSwap;
use macroquad::math::{f64, u64};
use macroquad::prelude::get_time;
use std::mem;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use rand::{Rng, SeedableRng};
use rand_xoshiro::Xoshiro256PlusPlus;
use crate::physics::module_diffusion_thermal::ModuleDiffusionThermal;
use crate::physics::module_transforms_thermal::ModuleTransformsThermal;

/// Generic double buffer over any T. We use it for `Vec<MaterialId>` and `Vec<Entity>`.
#[derive(Debug)]
pub struct DoubleBuffer<T> {
    pub cur: T,
    pub next: T,
}

impl<T: Clone> DoubleBuffer<T> {
    pub fn new(initial: T) -> Self {
        Self { cur: initial.clone(), next: initial }
    }

    /// Copy current into next so modules can make changes on top.
    pub fn sync(&mut self) {
        self.next.clone_from(&self.cur);
    }

    pub fn swap(&mut self) {
        mem::swap(&mut self.cur, &mut self.next);
    }
}

/// Empty placeholder for future entities class.
#[derive(Copy, Clone, Debug)]
pub struct Entity {

}

impl Entity {
    pub fn empty() -> Self {
        Self { }
    }
}

/// A snapshot of world state produced by the Sim thread and used by the Render thread.
pub struct Snapshot {
    pub w: usize,
    pub h: usize,
    pub cell_mat_ids: Box<[MaterialId]>,
    pub cell_temps: Box<[f32]>,
}

impl Snapshot {
    pub fn mat_id_at(&self, x: usize, y: usize) -> MaterialId {
        self.cell_mat_ids[y * self.w + x]
    }
    pub fn temp_at(&self, x: usize, y: usize) -> f32 {
        self.cell_temps[y * self.w + x]
    }
}

/// Stores data used by both the Sim thread and the Render thread.
pub struct Shared {
    pub current: ArcSwap<Snapshot>,
    pub mat_db: Arc<MaterialDb>,
    pub react_db: Arc<ReactionDb>,
    pub tick_count: AtomicU64,
}

impl Shared {
    pub fn new(initial: Arc<Snapshot>, mat_db: Arc<MaterialDb>, react_db: Arc<ReactionDb>) -> Arc<Self> {
        Arc::new(Self {
            current: ArcSwap::new(initial),
            mat_db,
            react_db,
            tick_count: AtomicU64::new(0),
        })
    }
}

/// Helper for keeping track of ticks per second.
pub struct TpsTracker {
    last_ticks: u64,
    last_time: f64,
    recent_tps: f64,
}

impl TpsTracker {
    pub fn new() -> Self {
        Self {
            last_ticks: 0,
            last_time: get_time(),
            recent_tps: 0.0,
        }
    }

    pub fn update(&mut self, shared: &Arc<Shared>) -> f64 {
        let now = get_time();
        let ticks = shared.tick_count.load(Ordering::Relaxed);

        let delta_time = now - self.last_time;
        let delta_ticks = ticks - self.last_ticks;

        if delta_time >= 1.0 {
            self.recent_tps = delta_ticks as f64 / delta_time;
            self.last_ticks = ticks;
            self.last_time = now;
        }

        self.recent_tps
    }
}

/// Builds world and physics engine.
pub fn build_world_and_engine(w: usize, h: usize, mat_db: &Arc<MaterialDb>, react_db: &Arc<ReactionDb>) -> (World, Engine) {
    let mut world = World::new(w, h, mat_db, react_db);
    let mut phys_eng = Engine::new(mat_db, w, h);

    let base_seed = 123456789u64;
    let mut global_rng = Xoshiro256PlusPlus::seed_from_u64(base_seed);

    // Basic random map
    {
        let (curr, mut next) = world.ctx_pair();

        // for y in 0..h {
        //     for x in 0..w {
        //         next.set_mat_id(x, y, curr.mat_db.get_id("base:insulation").unwrap());
        //         if (x % 10 == 0) && (y % 10 == 0) {
        //             next.set_temp(x, y, 10000000000.0);
        //         }
        //         else {
        //             next.set_temp(x, y, -10000000.0);
        //         }
        //     }
        // }

        // for y in 0..h {
        //     for x in 0..w {
        //         let result = global_rng.random_range(0.0..1.0);
        //         if result < 0.01 {
        //             next.set_mat_id(x, y, curr.mat_db.get_id("base:blood").unwrap());
        //             next.set_temp(x, y, 10000000.0);
        //         }
        //         else if result < 0.2 {
        //             next.set_mat_id(x, y, curr.mat_db.get_id("base:water").unwrap());
        //         }
        //         else if result < 0.25 {
        //             next.set_mat_id(x, y, curr.mat_db.get_id("base:lava").unwrap());
        //             next.set_temp(x, y, -2000000.0);
        //         }
        //         else {
        //             next.set_mat_id(x, y, curr.mat_db.get_id("base:air").unwrap());
        //         }
        //     }
        // }

        for y in 0..h {
            for x in 0..w {
                next.set_mat_id(x, y, curr.mat_db.get_id("base:water").unwrap());
            }
        }

        next.set_temp(0, 10, 100000.0);
        for x in 0..w {
            next.set_mat_id(x, 10, curr.mat_db.get_id("base:diamond").unwrap());
        }

        for y in 0..h {
            next.set_mat_id(60, y, curr.mat_db.get_id("base:diamond").unwrap());
            next.set_mat_id(120, y, curr.mat_db.get_id("base:diamond").unwrap());
            next.set_mat_id(180, y, curr.mat_db.get_id("base:diamond").unwrap());
            next.set_mat_id(240, y, curr.mat_db.get_id("base:diamond").unwrap());
        }

        next.set_temp(0, 22, 10000000000.0);
        next.set_temp(w-1, 22, -10000000000.0);
        for x in 0..w {
            next.set_mat_id(x, 21, curr.mat_db.get_id("base:insulation").unwrap());
            next.set_mat_id(x, 22, curr.mat_db.get_id("base:diamond").unwrap());
            next.set_mat_id(x, 23, curr.mat_db.get_id("base:insulation").unwrap());
        }

        world.swap_all();
    }

    // Physics modules
    {
        let (curr, mut next) = world.ctx_pair();

        // Modules are applied in the order they are added. Modules should be okay to run in any order.
        // However, by necessity it usually makes sense to run them in the following three stages:

        // Stage 1. Things that modify the state (i.e. temperature) of cells.
        phys_eng.add(ModuleDiffusionThermal::new(&curr,     base_seed ^ 0x0FEDCBA123456789));

        // Stage 2. Things that change the material of the cell.
        phys_eng.add(ModuleTransformsThermal::new(&curr,    base_seed ^ 0x345289A01DEFCB67));
        phys_eng.add(ModuleReactionsBasic::new(&curr,       base_seed ^ 0x0123456789ABCDEF));

        // Stage 3. Things that move cell contents around.
        // Cell swap intents should be applied last, because they usually want to swap state that was modified by other modules.
        // For instance, a moving steam particle should carry its temp with it, including changes to that temp this tick.
        // So we let all the thermal diffusion occur, then move the 'particle', so it can be ready for diffusion next frame.
        // To do so, it needs to swap the already modified values in next buffer.
        phys_eng.add(ModuleBehaviorSteam::new(&curr,        base_seed ^ 0xF0E1D2C3B4A59687));
    }
    (world, phys_eng)
}

/// Loads DBs, builds World and Phys Engine, starts the Sim thread, and
/// returns a handle to the Shared data struct for the Render thread.
pub fn spawn_sim_thread(w: usize, h: usize) -> Arc<Shared> {
    let mat_db = {
        let mut mdb = MaterialDb::new();
        mdb
            .load_ron_file("assets/materials_base.ron")
            .expect("failed to load materials");
        Arc::new(mdb)
    };
    let react_db = {
        let mut rdb = ReactionDb::new();
        (rdb)
            .load_ron_file(&mat_db, "assets/reactions_base.ron")
            .expect("failed to load reactions");
        Arc::new(rdb)
    };
    let initial = Arc::new(Snapshot {
        w,
        h,
        cell_mat_ids: vec![MaterialId(0); w * h].into_boxed_slice(),
        cell_temps: vec![0.0f32; w * h].into_boxed_slice(),
    });
    let shared = Shared::new(initial, mat_db, react_db);

    std::thread::spawn({
        let shared = Arc::clone(&shared);
        move || {

            let publish = |world: &World| {
                let snap = Snapshot {
                    w: world.w,
                    h: world.h,
                    cell_mat_ids: world.export_cell_mat_ids_boxed(),
                    cell_temps: world.export_cell_temps_boxed(),
                };
                shared.current.store(Arc::new(snap));
            };

            let (mut world, mut phys_eng) = build_world_and_engine(w, h, &shared.mat_db, &shared.react_db);

            publish(&world);

            loop {
                phys_eng.step(&mut world);
                shared.tick_count.fetch_add(1, Ordering::Relaxed);
                publish(&world);
            }
        }
    });

    shared
}