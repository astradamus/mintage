use crate::physics::module::{Module, ModuleOutput};
use crate::physics::util::rand_iter_dir;
use crate::world::CurrCtx;
use rand::SeedableRng;
use rand_xoshiro::Xoshiro256PlusPlus;
use serde_json::Value;
use std::collections::HashMap;
use crate::material::MaterialId;

#[inline(always)]
fn harmonic_mean(a: f32, b: f32) -> f32 {
    let s = a + b;
    if s == 0.0 { 0.0 } else { (2.0 * a * b) / s }
}

#[inline(always)]
fn calc_neighbor_flux(mat_ids: &[MaterialId], temps: &[f32], diff_of: &[f32], neighbor_index: usize, d_loc: f32, t_loc: f32) -> f32 {
    let id_n = mat_ids[neighbor_index];
    let t_n = temps[neighbor_index];
    let d_n = diff_of[id_n.0 as usize];
    let g = harmonic_mean(d_loc, d_n);
    g * (t_n - t_loc)
}

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
        let w = curr.w;
        let h = curr.h;
        let mat_ids = curr.get_mat_ids();
        let temps = curr.get_temps();
        let diff_of = curr.mat_db.get_diffusivity_of();

        let mut delta_temp = vec![0.0; w * h];

        rand_iter_dir(&mut self.rng, w, h, |x, y| {

            let i_loc = y * w + x;
            let id_loc = mat_ids[i_loc];
            let t_loc = temps[i_loc];
            let d_loc = diff_of[id_loc.0 as usize];

            let mut flux = 0.0;

            if x > 0        { flux += calc_neighbor_flux(mat_ids, temps, diff_of, i_loc - 1, d_loc, t_loc); }
            if x + 1 < w    { flux += calc_neighbor_flux(mat_ids, temps, diff_of, i_loc + 1, d_loc, t_loc); }
            if y > 0        { flux += calc_neighbor_flux(mat_ids, temps, diff_of, i_loc - w, d_loc, t_loc); }
            if y + 1 < h    { flux += calc_neighbor_flux(mat_ids, temps, diff_of, i_loc + w, d_loc, t_loc); }

            delta_temp[i_loc] += flux;
        });

        ModuleOutput::DeltaTemp { delta_temp }
    }
}
