use crate::physics::module::{Module, ModuleOutput};
use crate::physics::util::rand_iter_dir;
use crate::world::CurrCtx;
use rand::SeedableRng;
use rand_xoshiro::Xoshiro256PlusPlus;
use serde_json::Value;
use std::collections::HashMap;

/// Must not exceed 0.25!
const THERMAL_DIFFUSION_RATE: f32 = 0.15;

pub struct ModuleDiffusionThermal {
    rng: Xoshiro256PlusPlus,
}

impl ModuleDiffusionThermal {
    pub fn new(curr: &CurrCtx<'_>, rng_seed: u64) -> Self {
        Self  {
            rng: Xoshiro256PlusPlus::seed_from_u64(rng_seed),
        }
    }
}

impl Module for ModuleDiffusionThermal {

    fn apply_config(&mut self, config: &HashMap<String, Value>) {}

    fn run(&mut self, curr: &CurrCtx<'_>) -> ModuleOutput {

        let mut delta_temp = vec![0.0; curr.w * curr.h];

        rand_iter_dir(&mut self.rng, curr.w, curr.h, |x, y| {

            // Get material of this cell.
            // let mat = curr.get_mat_id(x, y);

            let temp_local = curr.get_temp(x, y);

            // Sum neighbor temps. Missing neighbors are treated as non-existent (no exchange).
            let mut sum = 0.0;
            let mut count = 0;

            if x > 0            { sum += curr.get_temp(x-1, y); count += 1; }
            if x + 1 < curr.w   { sum += curr.get_temp(x+1, y); count += 1; }
            if y > 0            { sum += curr.get_temp(x, y-1); count += 1; }
            if y + 1 < curr.h   { sum += curr.get_temp(x, y+1); count += 1; }

            // Apply diffusion.
            let delta = THERMAL_DIFFUSION_RATE * (sum - (count as f32) * temp_local);
            delta_temp[y * curr.w + x] += delta;
        });

        ModuleOutput::DeltaTemp { delta_temp }
    }
}
