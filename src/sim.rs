use crate::material::{MaterialDb, MaterialId};
use crate::physics::engine::Engine;
use crate::physics::module_behavior_steam::ModuleBehaviorSteam;
use crate::physics::module_reactions_basic::ModuleReactionsBasic;
use crate::reaction::ReactionDb;
use crate::world::World;
use arc_swap::ArcSwap;
use macroquad::math::{f64, u64};
use macroquad::prelude::get_time;
use std::{fs, mem};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use image::GenericImageView;
use serde_json::Value;
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

/// Helper for loading map state from a bitmap and a RON file.
/// A given hex code on the bitmap represents a mat_id and temp.
#[derive(serde::Deserialize)]
struct MapEntry {
    material: String,
    temperature: f32,
}

/// Builds world and physics engine.
pub fn build_world_and_engine(config: HashMap<String, Value>, w: usize, h: usize, mat_db: &Arc<MaterialDb>, react_db: &Arc<ReactionDb>) -> (World, Engine) {
    let mut world = World::new(w, h, mat_db, react_db);
    let mut phys_eng = Engine::new(config, w, h);

    let base_seed = 123456789u64;
    // let mut global_rng = Xoshiro256PlusPlus::seed_from_u64(base_seed);

    // Basic bitmap-based map loading for demo purposes.
    {
        let (curr, mut next) = world.ctx_pair();

        // Initialize the world before loading the png map.
        for y in 0..h {
            for x in 0..w {
                next.set_mat_id(x, y, curr.mat_db.get_id("base:air").expect("Missing material: base:air"));
                next.set_temp(x, y, 50.0);
            }
        }

        // The RON file assigns a mat_id and temp to a given color hex code.
        // We make a map of hex codes to map entries, then read the bitmap and assign.
        if let Ok(key_text) = fs::read_to_string("assets/map_key.ron") {
            if let Ok(mut map_key_raw) = ron::de::from_str::<HashMap<String, MapEntry>>(&key_text) {
                // Convert hex codes to uppercase so user can't get it wrong.
                let map_key = map_key_raw
                    .drain()
                    .map(|(k, v)| (k.to_uppercase(), v))
                    .collect::<HashMap<String, MapEntry>>();
                if let Ok(map_img) = image::open("assets/map.png") {
                    let (img_w, img_h) = map_img.dimensions();

                    // Clamp map size to world size.
                    for y in 0..img_h.min(h as u32) {
                        for x in 0..img_w.min(w as u32) {
                            let p = map_img.get_pixel(x, y);
                            let hex = format!("#{:02X}{:02X}{:02X}", p[0], p[1], p[2]);
                            if let Some(entry) = map_key.get(&hex) {
                                if let Some(mat_id) = mat_db.get_id(&entry.material) {
                                    next.set_mat_id(x as usize, y as usize, mat_id);
                                    next.set_temp(x as usize, y as usize, entry.temperature);
                                }
                            }
                        }
                    }
                }
            }
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
pub fn spawn_sim_thread(config: HashMap<String, Value>, w: usize, h: usize) -> Arc<Shared> {
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

            let (mut world, mut phys_eng) = build_world_and_engine(config, w, h, &shared.mat_db, &shared.react_db);

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