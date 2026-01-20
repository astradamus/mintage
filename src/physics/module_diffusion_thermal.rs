use crate::physics::module::{Module, ModuleOutput};
use crate::physics::util::rand_iter_dir;
use crate::world::{CurrCtx, PostRunCtx};
use rand::SeedableRng;
use rand_xoshiro::Xoshiro256PlusPlus;
use serde_json::Value;
use std::collections::HashMap;
use crate::material::{MaterialId};

#[inline(always)]
fn harmonic_mean(a: f32, b: f32) -> f32 {
    let s = a + b;
    if s == 0.0 { 0.0 } else { (2.0 * a * b) / s }
}

/// Module for thermal diffusion, using  values to determine heat flow.
pub struct ModuleDiffusionThermal {
    rng: Xoshiro256PlusPlus,

    /// Store conductance for every horizontal edge (neighbor pair) in the world.
    gx: Vec<f32>,
    /// Store conductance for every vertical edge (neighbor pair) in the world.
    gy: Vec<f32>,
}

impl ModuleDiffusionThermal {
    pub fn new(curr: &CurrCtx<'_>, rng_seed: u64) -> Self {
        let w = curr.w;
        let h = curr.h;
        let mat_ids = curr.get_mat_ids();
        let diff_of = curr.mat_db.get_diffusivity_lookup();

        let mut gx = vec![0.0; (w - 1) * h];
        let mut gy = vec![0.0; w * (h - 1)];

        // Calculate initial state of gx/gy (must check every edge).
        // Horizontal edges.
        for y in 0..h {
            for x in 0..(w - 1) {
                let i0 = y * w + x;
                let i1 = i0 + 1;
                let d0 = diff_of[mat_ids[i0].0 as usize];
                let d1 = diff_of[mat_ids[i1].0 as usize];
                gx[Self::gx_idx(x, y, w)] = harmonic_mean(d0, d1);
            }
        }

        // Vertical edges.
        for y in 0..h - 1 {
            for x in 0..w {
                let i0 = y * w + x;
                let i1 = i0 + w;
                let d0 = diff_of[mat_ids[i0].0 as usize];
                let d1 = diff_of[mat_ids[i1].0 as usize];
                gy[Self::gy_idx(x, y, w)] = harmonic_mean(d0, d1);
            }
        }

        Self  {
            rng: Xoshiro256PlusPlus::seed_from_u64(rng_seed),
            gx,
            gy,
        }
    }

    // Updates conductance for all four edges (between all for neighbors) of the given point.
    pub fn update_conductance_local(&mut self, w: usize, h: usize, i: usize, diff_of: &[f32], future_mat_ids: &[MaterialId]) {
        let (x, y) = (i % w, i / w);
        let d = diff_of[future_mat_ids[i].0 as usize];

        // North edge.
        if y > 0 {
            let i_n = i - w;
            let d_n = diff_of[future_mat_ids[i_n].0 as usize];
            self.gy[Self::gy_idx(x, y - 1, w)] = harmonic_mean(d, d_n);
        }

        // South edge.
        if y + 1 < h {
            let i_s = i + w;
            let d_s = diff_of[future_mat_ids[i_s].0 as usize];
            self.gy[Self::gy_idx(x, y, w)] = harmonic_mean(d, d_s);
        }

        // West edge.
        if x > 0 {
            let i_w = i - 1;
            let d_w = diff_of[future_mat_ids[i_w].0 as usize];
            self.gx[Self::gx_idx(x - 1, y, w)] = harmonic_mean(d, d_w);
        }

        // East edge.
        if x + 1 < w {
            let i_e = i + 1;
            let d_e = diff_of[future_mat_ids[i_e].0 as usize];
            self.gx[Self::gx_idx(x, y, w)] = harmonic_mean(d, d_e);
        }
    }

    #[inline(always)]
    fn gx_idx(x: usize, y: usize, w: usize) -> usize { y * (w - 1) + x }
    #[inline(always)]
    fn gy_idx(x: usize, y: usize, w: usize) -> usize { y * w + x}
}

impl Module for ModuleDiffusionThermal {

    fn apply_config(&mut self, config: &HashMap<String, Value>) {}

    fn run(&mut self, curr: &CurrCtx<'_>) -> ModuleOutput {
        let w = curr.w;
        let h = curr.h;
        let temps = curr.get_temps();

        let mut delta_temp = vec![0.0; w * h];

        rand_iter_dir(&mut self.rng, w, h, |x, y| {

            let i_loc = y * w + x;
            let t_loc = temps[i_loc];

            let mut flux = 0.0;

            // North flux.
            if y > 0 {
                let g_n = self.gy[Self::gy_idx(x, y - 1, w)];
                flux += g_n * (temps[i_loc - w] - t_loc);
            }

            // South flux.
            if y + 1 < h {
                let g_s = self.gy[Self::gy_idx(x, y, w)];
                flux += g_s * (temps[i_loc + w] - t_loc);
            }

            // West flux.
            if x > 0 {
                let g_w = self.gx[Self::gx_idx(x - 1, y, w)];
                flux += g_w * (temps[i_loc - 1] - t_loc);
            }

            // East flux.
            if x + 1 < w {
                let g_e = self.gx[Self::gx_idx(x, y, w)];
                flux += g_e * (temps[i_loc + 1] - t_loc);
            }

            delta_temp[i_loc] += flux;
        });

        ModuleOutput::DeltaTemp { delta_temp }
    }

    fn post_run(&mut self, post: &PostRunCtx<'_>, changed_cells: &[usize]) {
        let w = post.w;
        let h = post.h;
        let diff_of = post.mat_db.get_diffusivity_lookup();
        let mat_ids = post.next_cell_mat_ids;
        for &i in changed_cells {
            self.update_conductance_local(w, h, i, diff_of, mat_ids);
        }
    }
}
