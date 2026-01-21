use std::sync::Arc;
use crate::material::{MaterialDb, MaterialId};
use crate::reaction::ReactionDb;
use crate::sim::{DoubleBuffer, Entity};

pub struct World {
    pub w: usize,
    pub h: usize,

    pub cell_mat_ids: DoubleBuffer<Vec<MaterialId>>,
    pub cell_temps: DoubleBuffer<Vec<f32>>,
    pub entities: DoubleBuffer<Vec<Entity>>,

    pub mat_db: Arc<MaterialDb>,
    pub react_db: Arc<ReactionDb>,
}

impl World {
    pub fn new(w: usize, h: usize, mat_db: &Arc<MaterialDb>, react_db: &Arc<ReactionDb>) -> Self {
        let cell_mat_ids = vec![MaterialId(0); w * h];
        let cell_temps = vec![0.0f32; w * h];
        let entities = vec![Entity::empty(); w * h];

        Self {
            w, h,
            cell_mat_ids: DoubleBuffer::new(cell_mat_ids),
            cell_temps: DoubleBuffer::new(cell_temps),
            entities: DoubleBuffer::new(entities),
            mat_db: Arc::clone(mat_db),
            react_db: Arc::clone(react_db),
        }
    }

    pub fn sync_all(&mut self) {
        self.cell_mat_ids.sync();
        self.cell_temps.sync();
        self.entities.sync();
    }

    /// Remember that if you are manually calling this for any reason (like testing),
    /// you will probably also want to call `sync_all` after!
    pub fn swap_all(&mut self) {
        self.cell_mat_ids.swap();
        self.cell_temps.swap();
        self.entities.swap();
    }

    pub fn ctx_pair(&mut self) -> (CurrCtx<'_>, NextCtx<'_>) {
        let curr = CurrCtx {
            w: self.w,
            h: self.h,
            cell_mat_ids: &self.cell_mat_ids.cur,
            cell_temps: &self.cell_temps.cur,
            entities: &self.entities.cur,
            mat_db: &self.mat_db,
            react_db: &self.react_db,
        };
        let next = NextCtx {
            w: self.w,
            h: self.h,
            cell_mat_ids: &mut self.cell_mat_ids.next,
            cell_temps: &mut self.cell_temps.next,
            entities: &mut self.entities.next,
        };
        (curr, next)
    }

    pub fn ctx_post_run(&self) -> PostRunCtx<'_>{
        PostRunCtx {
            w: self.w,
            h: self.h,
            curr_cell_mat_ids: &self.cell_mat_ids.cur,
            next_cell_mat_ids: &self.cell_mat_ids.next,
            cell_temps: &self.cell_temps.cur,
            entities: &self.entities.cur,
            mat_db: &self.mat_db,
            react_db: &self.react_db,
        }
    }

    pub fn export_cell_mat_ids_boxed(&self) -> Box<[MaterialId]> {
        self.cell_mat_ids.cur.clone().into_boxed_slice()
    }

    pub fn export_cell_temps_boxed(&self) -> Box<[f32]> {
        self.cell_temps.cur.clone().into_boxed_slice()
    }
}



// ------------------------------ CURR FRAME CONTEXT -------------------------------
pub struct CurrCtx<'a> {
    pub w: usize,
    pub h: usize,
    pub cell_mat_ids: &'a [MaterialId],
    pub cell_temps: &'a [f32],
    pub entities: &'a [Entity],
    pub mat_db: &'a MaterialDb,
    pub react_db: &'a ReactionDb,
}

impl<'a> CurrCtx<'a> {

    #[inline] pub fn get_mat_ids(&self) -> &[MaterialId] {
        self.cell_mat_ids
    }

    #[inline] pub fn get_temps(&self) -> &[f32] {
        self.cell_temps
    }

    #[inline] pub fn get_mat_id(&self, x: usize, y: usize) -> MaterialId {
        self.cell_mat_ids[index(self.w, x, y)]
    }
    #[inline] pub fn get_mat_id_i(&self, i: usize) -> MaterialId {
        self.cell_mat_ids[i]
    }

    #[inline] pub fn get_temp(&self, x: usize, y: usize) -> f32 {
        self.cell_temps[index(self.w, x, y)]
    }

    #[inline] pub fn get_temp_i(&self, i: usize) -> f32 {
        self.cell_temps[i]
    }

    pub fn contains(&self, x: isize, y: isize) -> bool {
        contains(self.w, self.h, x as usize, y as usize)
    }
}



// ------------------------------ NEXT FRAME CONTEXT ------------------------------

pub struct NextCtx<'a> {
    w: usize,
    h: usize,
    cell_mat_ids: &'a mut Vec<MaterialId>,
    cell_temps: &'a mut Vec<f32>,
    entities: &'a mut Vec<Entity>,
}

impl<'a> NextCtx<'a> {
    #[inline] pub fn set_mat_id(&mut self, x: usize, y: usize, material_id: MaterialId) {
        self.cell_mat_ids[index(self.w, x, y)] = material_id;
    }

    #[inline] pub fn set_temp(&mut self, x: usize, y: usize, temp: f32) {
        self.cell_temps[index(self.w, x, y)] = temp;
    }

    /// Uses flattened index, which is sometimes faster than converting to 2D and back.
    #[inline] pub fn set_temp_i(&mut self, i: usize, temp: f32) {
        self.cell_temps[i] = temp;
    }

    #[inline] pub fn add_temp(&mut self, x: usize, y: usize, temp: f32) {
        self.cell_temps[index(self.w, x, y)] += temp;
    }

    /// Uses flattened index, which is sometimes faster than converting to 2D and back.
    #[inline] pub fn add_temp_i(&mut self, i: usize, temp: f32) {
        self.cell_temps[i] += temp;
    }

    #[inline] pub fn peek_future_temp(&self, x: usize, y: usize) -> f32 {
        self.cell_temps[index(self.w, x, y)]
    }
}

// ------------------------------- POST RUN CONTEXT -------------------------------

pub struct PostRunCtx<'a> {
    pub w: usize,
    pub h: usize,
    pub curr_cell_mat_ids: &'a [MaterialId],
    pub next_cell_mat_ids: &'a [MaterialId],
    pub cell_temps: &'a [f32],
    pub entities: &'a [Entity],
    pub mat_db: &'a MaterialDb,
    pub react_db: &'a ReactionDb,
}

// -------------------------------- UTILITIES --------------------------------
/// Convert a 2D index to 1D.
#[inline] fn index(w: usize, x: usize, y: usize) -> usize { y * w + x }

#[inline] fn contains(w: usize, h: usize, x: usize, y: usize) -> bool {
    x < w && y < h
}