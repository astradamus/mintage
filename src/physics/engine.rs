use crate::physics::intent::CellIntent;
use crate::physics::module::{Module, ModuleOutput};
use crate::world::{CurrCtx, NextCtx, World};
use rayon::prelude::{IntoParallelRefMutIterator, ParallelIterator};
use serde_json::Value;
use std::collections::HashMap;

pub struct Engine {
    modules: Vec<Box<dyn Module + Send>>,
    config: HashMap<String, Value>,
    changed_dense: Vec<bool>,
    changed_sparse: Vec<usize>,
}

impl Engine {
    pub fn new(config: HashMap<String, Value>, world_w: usize, world_h: usize) -> Self {
        Self {
            modules: vec![],
            config,
            changed_dense: vec![false; world_w * world_h],
            changed_sparse: vec![],
        }
    }

    pub fn add<M: Module + 'static>(&mut self, mut m: M) {
        m.apply_config(&self.config);
        self.modules.push(Box::new(m));
    }

    pub fn step(&mut self, world: &mut World) {

        // Copy curr buffer to next buffer.
        world.sync_all();

        // Get world contexts.
        let (curr, mut next) = world.ctx_pair();

        // Gather intents from modules in parallel.
        // Gather order is deterministic within modules.
        // Intents are applied in the same order as they were gathered.
        // Earlier intents apply first, blocking later ones.
        let outputs: Vec<ModuleOutput> = self.modules
            .par_iter_mut()
            .map(|m| m.run(&curr))
            .collect();

        // Apply outputs in module order to preserve determinism.
        for out in outputs {
            match out {
                ModuleOutput::CellIntents { intents } => {
                    self.apply_intents(&curr, &mut next, &intents);
                }
                ModuleOutput::DeltaTemp { delta_temp } => {
                    self.apply_delta_temp(&curr, &mut next, &delta_temp);
                }
            }
        }

        // Get post-run context.
        let post = world.ctx_post_run();

        // Let modules run post-step updates. Some modules want to pre-compute values for speed.
        self.modules
            .par_iter_mut()
            .for_each(|m| m.post_run(&post, self.changed_sparse.as_slice()));

        // Reset changed flags for next frame.
        for &i in &self.changed_sparse {
            self.changed_dense[i] = false;
        }
        self.changed_sparse.clear();

        // Commit the frame.
        world.swap_all();
    }

    fn apply_intents(&mut self, curr: &CurrCtx<'_>, next: &mut NextCtx<'_>, intents: &[CellIntent]) {

        for intent in intents {
            let cells = intent.affected_cells();

            // Check if any involved cell was already changed this frame.
            if cells.iter().any(|(x, y)| self.changed_dense[y * curr.w + x]) {
                continue; // Skip this intent due to conflict with previous intent.
            }

            // Mark cells as changed.
            for (x, y) in &cells {
                let i = y * curr.w + x;
                self.changed_dense[i] = true;
                self.changed_sparse.push(i);
            }

            // Apply the action.
            match intent {
                &CellIntent::Transform { cell, out } => {
                    next.set_mat_id(cell.0, cell.1, out);
                },
                &CellIntent::Reaction { cell_a, cell_b, out_a, out_b } => {
                    next.set_mat_id(cell_a.0, cell_a.1, out_a);
                    next.set_mat_id(cell_b.0, cell_b.1, out_b);
                },
                &CellIntent::MoveSwap { from, to } => {
                    let mat_from = curr.get_mat_id(from.0, from.1);
                    let mat_to = curr.get_mat_id(to.0, to.1);
                    next.set_mat_id(from.0, from.1, mat_to);
                    next.set_mat_id(to.0, to.1, mat_from);

                    // Peek future temp, because we want to make sure temp changes affect this particle.
                    // Note that we do not peek future mat id. That's because mat id changes
                    // prevent move swap intents from being applied in the same tick.
                    let temp_from = next.peek_future_temp(from.0, from.1);
                    let temp_to = next.peek_future_temp(to.0, to.1);
                    next.set_temp(from.0, from.1, temp_to);
                    next.set_temp(to.0, to.1, temp_from);
                },
            }
        }
    }

    fn apply_delta_temp(&self, curr: &CurrCtx<'_>, next: &mut NextCtx<'_>, delta_temp: &[f32]) {
        for i in 0..(curr.w * curr.h) {
            next.add_temp_i(i, delta_temp[i]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::material::MaterialDb;
    use crate::reaction::ReactionDb;
    use crate::world::World;
    use crate::world::PostRunCtx;
    use std::sync::Arc;
    use std::sync::Mutex;

    /// Mock module that always returns a fixed intent on the first step (and nothing thereafter).
    /// Also, tracks when post-run is called and the changed cells it returned.
    struct MockModule {
        output: Option<ModuleOutput>,
        post_run_called: Arc<Mutex<bool>>,
        received_changed_cells: Arc<Mutex<Vec<usize>>>,
    }

    impl MockModule {
        fn new(output: Option<ModuleOutput>) -> Self {
            Self {
                output,
                post_run_called: Arc::new(Mutex::new(false)),
                received_changed_cells: Arc::new(Mutex::new(vec![])),
            }
        }
    }

    impl Module for MockModule {
        fn apply_config(&mut self, _config: &HashMap<String, Value>) {}
        /// Keep in mind that the intent only fires one time, after that the module returns nothing.
        fn run(&mut self, _curr: &CurrCtx<'_>) -> ModuleOutput {
            self.output.take().unwrap_or(ModuleOutput::CellIntents { intents: vec![] })
        }
        fn post_run(&mut self, _post: &PostRunCtx<'_>, changed_cells: &[usize]) {
            *self.post_run_called.lock().unwrap() = true;
            *self.received_changed_cells.lock().unwrap() = changed_cells.to_vec();
        }
    }

    /// Test helper.
    fn mock_world(w: usize, h: usize) -> (World, Arc<MaterialDb>) {
        let mut mat_db = MaterialDb::new();
        mat_db.load_ron_str(r#"
            {
                "test:air": (),
                "test:water": (),
                "test:rock": (),
            }
        "#).unwrap();
        let mat_db = Arc::new(mat_db);
        let react_db = Arc::new(ReactionDb::new());
        let mut world = World::new(w, h, &mat_db, &react_db);
        {
            let (_, mut next) = world.ctx_pair();

            let mat_id_air = mat_db.get_id("test:air").unwrap();
            next.set_mat_id(0, 0, mat_id_air);
            next.set_mat_id(0, 1, mat_id_air);
            next.set_mat_id(1, 0, mat_id_air);
            next.set_mat_id(1, 1, mat_id_air);
            world.swap_all();
            world.sync_all();
        }
        (world, mat_db)
    }

    #[test]
    fn test_mock_world() {
        let (world, mat_db) = mock_world(2, 2);
        assert_eq!(mat_db.get_mat_count(), 3);
        assert_eq!(world.w, 2);
        assert_eq!(world.h, 2);

        let mat_id_air = mat_db.get_id("test:air").unwrap();
        assert_eq!(world.cell_mat_ids.cur[0], mat_id_air);
        assert_eq!(world.cell_mat_ids.cur[1], mat_id_air);
        assert_eq!(world.cell_mat_ids.cur[2], mat_id_air);
        assert_eq!(world.cell_mat_ids.cur[3], mat_id_air);

        assert_eq!(world.cell_temps.cur[0], 0.0);
        assert_eq!(world.cell_temps.cur[1], 0.0);
        assert_eq!(world.cell_temps.cur[2], 0.0);
        assert_eq!(world.cell_temps.cur[3], 0.0);
    }


    // Intention tests.
    #[test]
    fn test_engine_intent_transform() {
        let (mut world, mat_db) = mock_world(2, 2);
        let mut engine = Engine::new(HashMap::new(), 2, 2);

        let mat_id_air = mat_db.get_id("test:air").unwrap();
        let mat_id_water = mat_db.get_id("test:water").unwrap();

        let intent = CellIntent::Transform {
            cell: (1, 1),
            out: mat_id_water,
        };

        // Add module and step engine.
        engine.add(MockModule::new(Some(ModuleOutput::CellIntents { intents: vec![intent] })));
        engine.step(&mut world);

        // Ensure (1,1) is now water. Other cells unchanged.
        assert_eq!(world.cell_mat_ids.cur[0], mat_id_air);
        assert_eq!(world.cell_mat_ids.cur[1], mat_id_air);
        assert_eq!(world.cell_mat_ids.cur[2], mat_id_air);
        assert_eq!(world.cell_mat_ids.cur[3], mat_id_water);
    }

    #[test]
    fn test_engine_intent_reaction() {
        let (mut world, mat_db) = mock_world(2, 2);
        let mut engine = Engine::new(HashMap::new(), 2, 2);

        let mat_id_air = mat_db.get_id("test:air").unwrap();
        let mat_id_water = mat_db.get_id("test:water").unwrap();

        let intent = CellIntent::Reaction {
            cell_a: (1, 0),
            cell_b: (0, 1),
            out_a: mat_id_water,
            out_b: mat_id_water,
        };

        // Add module and step engine.
        engine.add(MockModule::new(Some(ModuleOutput::CellIntents { intents: vec![intent] })));
        engine.step(&mut world);

        // Ensure (0,1) and (1,0) are now water. Other cells unchanged.
        assert_eq!(world.cell_mat_ids.cur[0], mat_id_air);
        assert_eq!(world.cell_mat_ids.cur[1], mat_id_water);
        assert_eq!(world.cell_mat_ids.cur[2], mat_id_water);
        assert_eq!(world.cell_mat_ids.cur[3], mat_id_air);
    }

    #[test]
    fn test_engine_intent_move_swap() {
        let (mut world, mat_db) = mock_world(2, 2);
        let mut engine = Engine::new(HashMap::new(), 2, 2);

        let mat_id_air = mat_db.get_id("test:air").unwrap();
        let mat_id_water = mat_db.get_id("test:water").unwrap();
        let mat_id_rock = mat_db.get_id("test:rock").unwrap();

        {
            let (_, mut next) = world.ctx_pair();
            next.set_mat_id(0, 0, mat_id_rock);     // Rock at (0,0).
            next.set_temp(0, 0, 500.0);
            next.set_mat_id(1, 0, mat_id_water);    // Water at (1,0).
            next.set_temp(1, 0, -20.0);
            world.swap_all();
            world.sync_all();
        }

        // Add module and step engine.
        engine.add(MockModule::new(Some(ModuleOutput::CellIntents {
            intents: vec![CellIntent::MoveSwap { from: (0, 0), to: (1, 0) }]
        })));
        engine.step(&mut world);

        // Ensure swap included both material and temperature.
        assert_eq!(world.cell_mat_ids.cur[0], mat_id_water);
        assert_eq!(world.cell_mat_ids.cur[1], mat_id_rock);
        assert_eq!(world.cell_mat_ids.cur[2], mat_id_air);
        assert_eq!(world.cell_mat_ids.cur[3], mat_id_air);

        assert_eq!(world.cell_temps.cur[0], -20.0);
        assert_eq!(world.cell_temps.cur[1], 500.0);
        assert_eq!(world.cell_temps.cur[2], 0.0);
        assert_eq!(world.cell_temps.cur[3], 0.0);
    }

    #[test]
    fn test_engine_delta_temp() {
        let (mut world, _) = mock_world(2, 2);
        let mut engine = Engine::new(HashMap::new(), 2, 2);

        let delta_temp = vec![1.0, 2.0, 3.0, 4.0];
        engine.add(MockModule::new(Some(ModuleOutput::DeltaTemp { delta_temp })));

        engine.step(&mut world);

        assert_eq!(world.cell_temps.cur[0], 1.0);
        assert_eq!(world.cell_temps.cur[1], 2.0);
        assert_eq!(world.cell_temps.cur[2], 3.0);
        assert_eq!(world.cell_temps.cur[3], 4.0);
    }


// Conflict/overlap resolution tests.

    // Test each intent type with itself.
    #[test]
    fn test_engine_intent_transform_twice_conflict() {
        let (mut world, mat_db) = mock_world(2, 2);
        let mut engine = Engine::new(HashMap::new(), 2, 2);

        let mat_id_water = mat_db.get_id("test:water").unwrap();
        let mat_id_rock = mat_db.get_id("test:rock").unwrap();

        // First module transforms (1,0) to Water.
        engine.add(MockModule::new(Some(ModuleOutput::CellIntents {
            intents: vec![CellIntent::Transform { cell: (1, 0), out: mat_id_water }]
        })));

        // Second module transforms (1,0) to Rock. Should be ignored/discarded.
        engine.add(MockModule::new(Some(ModuleOutput::CellIntents {
            intents: vec![CellIntent::Transform { cell: (1, 0), out: mat_id_rock }]
        })));

        engine.step(&mut world);

        // First module takes priority, so (1,0) is now Water.
        assert_eq!(world.cell_mat_ids.cur[1], mat_id_water);
    }

    #[test]
    fn test_engine_intent_reaction_twice_conflict() {
        let (mut world, mat_db) = mock_world(2, 2);
        let mut engine = Engine::new(HashMap::new(), 2, 2);

        let mat_id_air = mat_db.get_id("test:air").unwrap();
        let mat_id_water = mat_db.get_id("test:water").unwrap();
        let mat_id_rock = mat_db.get_id("test:rock").unwrap();

        // First module reacts (0,1) and (1,0) to Water.
        engine.add(MockModule::new(Some(ModuleOutput::CellIntents {
            intents: vec![CellIntent::Reaction { cell_a: (0, 1), cell_b: (1, 0), out_a: mat_id_water, out_b: mat_id_water }]
        })));

        // Second module reacts (1,0) and (1,1) to Rock. BOTH should be ignored/discarded, as reactions are atomic.
        engine.add(MockModule::new(Some(ModuleOutput::CellIntents {
            intents: vec![CellIntent::Reaction { cell_a: (1, 0), cell_b: (1, 1), out_a: mat_id_rock, out_b: mat_id_rock }]
        })));

        engine.step(&mut world);

        // First module takes priority, so (0,1) and (1,0) is now Water.
        assert_eq!(world.cell_mat_ids.cur[0], mat_id_air);
        assert_eq!(world.cell_mat_ids.cur[1], mat_id_water);
        assert_eq!(world.cell_mat_ids.cur[2], mat_id_water);
        assert_eq!(world.cell_mat_ids.cur[3], mat_id_air); // Shouldn't change to rock because reaction should be blocked.
    }

    #[test]
    fn test_engine_intent_move_twice_conflict() {
        let (mut world, mat_db) = mock_world(2, 2);
        let mut engine = Engine::new(HashMap::new(), 2, 2);

        let mat_id_air = mat_db.get_id("test:air").unwrap();
        let mat_id_water = mat_db.get_id("test:water").unwrap();
        let mat_id_rock = mat_db.get_id("test:rock").unwrap();

        {
            let (_, mut next) = world.ctx_pair();
            next.set_mat_id(0, 0, mat_id_water);   // Water at (0,0).
            next.set_temp(0, 0, 400.0);
            next.set_mat_id(1, 0, mat_id_rock);    // Rock at (1,0).
            next.set_temp(1, 0, -20.0);
            world.swap_all();
        }

        // First module swaps (0,0) and (1,0).
        engine.add(MockModule::new(Some(ModuleOutput::CellIntents {
            intents: vec![CellIntent::MoveSwap { from: (0, 0), to: (1, 0) }]
        })));

        // Second module swaps (1,0) and (1,1). Should be ignored/discarded.
        engine.add(MockModule::new(Some(ModuleOutput::CellIntents {
            intents: vec![CellIntent::MoveSwap { from: (0, 0), to: (1, 0) }]
        })));

        engine.step(&mut world);

        // First module takes priority, so (0,1) and (1,0) swapped, but (1,0) and (1,1) did not.
        assert_eq!(world.cell_mat_ids.cur[0], mat_id_rock);
        assert_eq!(world.cell_mat_ids.cur[1], mat_id_water);
        assert_eq!(world.cell_mat_ids.cur[2], mat_id_air);
        assert_eq!(world.cell_mat_ids.cur[3], mat_id_air);
        assert_eq!(world.cell_temps.cur[0], -20.0);
        assert_eq!(world.cell_temps.cur[1], 400.0);
        assert_eq!(world.cell_temps.cur[2], 0.0);
        assert_eq!(world.cell_temps.cur[3], 0.0);
    }

    #[test]
    fn test_engine_delta_temp_twice_overlap() {
        let (mut world, _) = mock_world(2, 2);
        let mut engine = Engine::new(HashMap::new(), 2, 2);

        // Both modules add to temp of each cell.
        let delta_temp = vec![1.0, 2.0, 3.0, 4.0];
        engine.add(MockModule::new(Some(ModuleOutput::DeltaTemp { delta_temp })));
        let delta_temp = vec![10.0, 20.0, 30.0, 40.0];
        engine.add(MockModule::new(Some(ModuleOutput::DeltaTemp { delta_temp })));

        engine.step(&mut world);

        // Both modules should successfully add together.
        assert_eq!(world.cell_temps.cur[0], 11.0);
        assert_eq!(world.cell_temps.cur[1], 22.0);
        assert_eq!(world.cell_temps.cur[2], 33.0);
        assert_eq!(world.cell_temps.cur[3], 44.0);
    }

    // Test movement after other intents.
    #[test]
    fn test_engine_intent_transform_move_conflict() {
        let (mut world, mat_db) = mock_world(2, 2);
        let mut engine = Engine::new(HashMap::new(), 2, 2);

        let mat_id_air = mat_db.get_id("test:air").unwrap();
        let mat_id_water = mat_db.get_id("test:water").unwrap();

        // First module transforms (1,0) to Water.
        engine.add(MockModule::new(Some(ModuleOutput::CellIntents {
            intents: vec![CellIntent::Transform { cell: (1, 0), out: mat_id_water }]
        })));

        // Second module swaps (1,0) and (0,0).
        engine.add(MockModule::new(Some(ModuleOutput::CellIntents {
            intents: vec![CellIntent::MoveSwap { from: (1, 0), to: (0, 0) }]
        })));

        engine.step(&mut world);

        // Swap should fail because material changes and swaps cannot happen same frame.
        assert_eq!(world.cell_mat_ids.cur[0], mat_id_air);
        assert_eq!(world.cell_mat_ids.cur[1], mat_id_water);
    }

    #[test]
    fn test_engine_intent_reaction_move_conflict() {
        let (mut world, mat_db) = mock_world(2, 2);
        let mut engine = Engine::new(HashMap::new(), 2, 2);

        let mat_id_air = mat_db.get_id("test:air").unwrap();
        let mat_id_water = mat_db.get_id("test:water").unwrap();

        // First module reacts (1,0) and (0,1) to Water.
        engine.add(MockModule::new(Some(ModuleOutput::CellIntents {
            intents: vec![CellIntent::Reaction { cell_a: (1, 0), cell_b: (0, 1), out_a: mat_id_water, out_b: mat_id_water }]
        })));

        // Second module swaps (1,0) and (0,0). Should be ignored/discarded.
        engine.add(MockModule::new(Some(ModuleOutput::CellIntents {
            intents: vec![CellIntent::MoveSwap { from: (1, 0), to: (0, 0) }]
        })));

        engine.step(&mut world);

        // Swap should fail because material changes and swaps cannot happen same frame.
        assert_eq!(world.cell_mat_ids.cur[0], mat_id_air);
        assert_eq!(world.cell_mat_ids.cur[1], mat_id_water);
        assert_eq!(world.cell_mat_ids.cur[2], mat_id_water);
        assert_eq!(world.cell_mat_ids.cur[3], mat_id_air);
    }

    #[test]
    fn test_engine_intent_temp_move_overlap() {
        let (mut world, _) = mock_world(2, 2);
        let mut engine = Engine::new(HashMap::new(), 2, 2);

        // First module adds 500.0 to (1,0)'s temperature.
        let delta_temp = vec![0.0, 500.0, 0.0, 0.0];
        engine.add(MockModule::new(Some(ModuleOutput::DeltaTemp { delta_temp })));

        // Second module swaps (1,0) and (0,0).
        engine.add(MockModule::new(Some(ModuleOutput::CellIntents {
            intents: vec![CellIntent::MoveSwap { from: (1, 0), to: (0, 0) }]
        })));

        engine.step(&mut world);

        // The temp change should apply successfully and then be swapped in the same frame (overlap, not conflict).
        assert_eq!(world.cell_temps.cur[0], 500.0);
        assert_eq!(world.cell_temps.cur[1], 0.0);
    }

    // Test temps with others.
    #[test]
    fn test_engine_intent_transform_temp_overlap() {
        let (mut world, mat_db) = mock_world(2, 2);
        let mut engine = Engine::new(HashMap::new(), 2, 2);

        let mat_id_water = mat_db.get_id("test:water").unwrap();

        // First module transforms (1,0) to Water.
        engine.add(MockModule::new(Some(ModuleOutput::CellIntents {
            intents: vec![CellIntent::Transform { cell: (1, 0), out: mat_id_water }]
        })));

        // Second module adds 500.0 to (1,0)'s temperature.
        let delta_temp = vec![0.0, 500.0, 0.0, 0.0];
        engine.add(MockModule::new(Some(ModuleOutput::DeltaTemp { delta_temp })));

        engine.step(&mut world);

        // Both modules should apply their effects successfully (overlap, not conflict).
        assert_eq!(world.cell_mat_ids.cur[1], mat_id_water);
        assert_eq!(world.cell_temps.cur[1], 500.0);
    }

    #[test]
    fn test_engine_intent_reaction_temp_overlap() {
        let (mut world, mat_db) = mock_world(2, 2);
        let mut engine = Engine::new(HashMap::new(), 2, 2);

        let mat_id_water = mat_db.get_id("test:water").unwrap();

        // First module reacts (0,1) and (1,0) to Water.
        engine.add(MockModule::new(Some(ModuleOutput::CellIntents {
            intents: vec![CellIntent::Reaction { cell_a: (0, 1), cell_b: (1, 0), out_a: mat_id_water, out_b: mat_id_water }]
        })));

        // Second module adds 500.0 to (0,1) temp and 400.0 to (1,0) temp.
        let delta_temp = vec![0.0, 500.0, 400.0, 0.0];
        engine.add(MockModule::new(Some(ModuleOutput::DeltaTemp { delta_temp })));

        engine.step(&mut world);

        // Both modules should apply their effects successfully (overlap, not conflict).
        assert_eq!(world.cell_mat_ids.cur[1], mat_id_water);
        assert_eq!(world.cell_mat_ids.cur[2], mat_id_water);
        assert_eq!(world.cell_temps.cur[1], 500.0);
        assert_eq!(world.cell_temps.cur[2], 400.0);
    }

    // Test transform followed by reaction.
    #[test]
    fn test_engine_intent_transform_reaction_conflict() {
        let (mut world, mat_db) = mock_world(2, 2);
        let mut engine = Engine::new(HashMap::new(), 2, 2);

        let mat_id_air = mat_db.get_id("test:air").unwrap();
        let mat_id_water = mat_db.get_id("test:water").unwrap();
        let mat_id_rock = mat_db.get_id("test:rock").unwrap();

        // First module transforms (1,0) to Water.
        engine.add(MockModule::new(Some(ModuleOutput::CellIntents {
            intents: vec![CellIntent::Transform { cell: (1, 0), out: mat_id_water }]
        })));

        // Second module reacts (1,0) and (1,1) to Rock. BOTH should be ignored/discarded, as reactions are atomic.
        engine.add(MockModule::new(Some(ModuleOutput::CellIntents {
            intents: vec![CellIntent::Reaction { cell_a: (1, 0), cell_b: (1, 1), out_a: mat_id_rock, out_b: mat_id_rock }]
        })));

        engine.step(&mut world);

        // First module takes priority, so (1,0) is now Water, nothing else changes.
        assert_eq!(world.cell_mat_ids.cur[0], mat_id_air);
        assert_eq!(world.cell_mat_ids.cur[1], mat_id_water);
        assert_eq!(world.cell_mat_ids.cur[2], mat_id_air);
        assert_eq!(world.cell_mat_ids.cur[3], mat_id_air);
    }

    // Post-run tests.
    #[test]
    fn test_engine_post_run_changed_cells_transform() {
        let (mut world, mat_db) = mock_world(2, 2);
        let mut engine = Engine::new(HashMap::new(), 2, 2);

        let mat_id_water = mat_db.get_id("test:water").unwrap();

        let mock = MockModule::new(Some(ModuleOutput::CellIntents {
            intents: vec![CellIntent::Transform { cell: (1, 1), out: mat_id_water }]
        }));

        let post_run_called = Arc::clone(&mock.post_run_called);
        let received_changed_cells = Arc::clone(&mock.received_changed_cells);

        // Add module and step engine.
        engine.add(mock);
        engine.step(&mut world);

        // Ensure post-run was called and changed cells are correct.
        assert!(*post_run_called.lock().unwrap());
        let changed = received_changed_cells.lock().unwrap();
        assert_eq!(changed.len(), 1);
        assert!(changed.contains(&3)); // (1,1) in 2x2 is index 3.
    }

    #[test]
    fn test_engine_post_run_changed_cells_reaction() {
        let (mut world, mat_db) = mock_world(2, 2);
        let mut engine = Engine::new(HashMap::new(), 2, 2);

        let mat_id_water = mat_db.get_id("test:water").unwrap();

        // First module reacts (0,1) and (1,0) to Water.
        let mock = MockModule::new(Some(ModuleOutput::CellIntents {
            intents: vec![CellIntent::Reaction { cell_a: (0, 1), cell_b: (1, 0), out_a: mat_id_water, out_b: mat_id_water}]
        }));

        let post_run_called = Arc::clone(&mock.post_run_called);
        let received_changed_cells = Arc::clone(&mock.received_changed_cells);

        // Add module and step engine.
        engine.add(mock);
        engine.step(&mut world);

        // Ensure post-run was called and changed cells are correct.
        assert!(*post_run_called.lock().unwrap());
        let changed = received_changed_cells.lock().unwrap();
        assert_eq!(changed.len(), 2);
        assert!(changed.contains(&1)); // (0,1) in 2x2 is index 1.
        assert!(changed.contains(&2)); // (1,0) in 2x2 is index 2.
    }

    #[test]
    fn test_engine_post_run_changed_cells_swap() {
        let (mut world, _) = mock_world(2, 2);
        let mut engine = Engine::new(HashMap::new(), 2, 2);

        // First module swaps (1,1) and (0,0).
        let mock = MockModule::new(Some(ModuleOutput::CellIntents {
            intents: vec![CellIntent::MoveSwap { from: (1, 1), to: (0, 0)}]
        }));

        let post_run_called = Arc::clone(&mock.post_run_called);
        let received_changed_cells = Arc::clone(&mock.received_changed_cells);

        // Add module and step engine.
        engine.add(mock);
        engine.step(&mut world);

        // Ensure post-run was called and changed cells are correct.
        assert!(*post_run_called.lock().unwrap());
        let changed = received_changed_cells.lock().unwrap();
        assert_eq!(changed.len(), 2);
        assert!(changed.contains(&0)); // (0,0) in 2x2 is index 0.
        assert!(changed.contains(&3)); // (1,1) in 2x2 is index 3.
    }
}
