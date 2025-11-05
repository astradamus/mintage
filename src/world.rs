use std::sync::Arc;
use macroquad::logging::warn;
use crate::material::{Material, MaterialDb, MaterialId};
use crate::reaction::ReactionDb;
use crate::sim::{DoubleBuffer, Entity};

pub struct World {
    pub w: usize,
    pub h: usize,

    pub cell_mat_ids: DoubleBuffer<Vec<MaterialId>>,
    pub entities: DoubleBuffer<Vec<Entity>>,

    pub mat_db: Arc<MaterialDb>,
    pub react_db: Arc<ReactionDb>,
}

impl World {
    pub fn new(w: usize, h: usize, mat_db: &Arc<MaterialDb>, react_db: &Arc<ReactionDb>) -> Self {
        let cell_mat_ids = vec![MaterialId(0); w * h];
        let entities = vec![Entity::empty(); w * h];

        Self {
            w, h,
            cell_mat_ids: DoubleBuffer::new(cell_mat_ids),
            entities: DoubleBuffer::new(entities),
            mat_db: Arc::clone(mat_db),
            react_db: Arc::clone(react_db),
        }
    }

    pub fn sync_all(&mut self) {
        self.cell_mat_ids.sync();
        self.entities.sync();
    }

    pub fn swap_all(&mut self) {
        self.cell_mat_ids.swap();
        self.entities.swap();
    }

    pub fn get_curr_mat_id_at(&self, x: usize, y: usize) -> Option<&MaterialId> {
        if let Some(cell) = self.cell_mat_ids.cur.get(index(self.w, x, y)) {
            Some(cell)
        }
        else {
            warn!("tried get_curr_mat_id_at for out-of-bounds cell: ({x}, {y})");
            None
        }
    }

    pub fn get_curr_mat_at(&self, x: usize, y: usize) -> Option<&Material> {
        if let Some(id) = self.get_curr_mat_id_at(x, y) {
            self.mat_db.get(id)
        }
        else {
            None
        }
    }

    pub fn ctx_pair(&mut self) -> (CurrCtx<'_>, NextCtx<'_>) {
        let curr = CurrCtx {
            w: self.w,
            h: self.h,
            cell_mat_ids: &self.cell_mat_ids.cur,
            entities: &self.entities.cur,
            mat_db: &self.mat_db,
            react_db: &self.react_db,
        };
        let next = NextCtx {
            w: self.w,
            h: self.h,
            cell_mat_ids: &mut self.cell_mat_ids.next,
            entities: &mut self.entities.next,
        };
        (curr, next)
    }

    pub fn export_cell_mat_ids_boxed(&self) -> Box<[MaterialId]> {
        self.cell_mat_ids.cur.clone().into_boxed_slice()
    }
}



// ------------------------------ CURR FRAME CONTEXT -------------------------------
pub struct CurrCtx<'a> {
    pub w: usize,
    pub h: usize,
    pub cell_mat_ids: &'a [MaterialId],
    pub entities: &'a [Entity],
    pub mat_db: &'a MaterialDb,
    pub react_db: &'a ReactionDb,
}

impl<'a> CurrCtx<'a> {
    #[inline] pub fn get_mat_id(&self, x: usize, y: usize) -> MaterialId {
        self.cell_mat_ids[index(self.w, x, y)]
    }

    pub fn contains(&self, x: isize, y: isize) -> bool {
        contains(self.w, self.h, x as usize, y as usize)
    }
}



// ------------------------------ NEXT FRAME CONTEXT ------------------------------

pub struct NextCtx<'a> {
    pub w: usize,
    pub h: usize,
    pub cell_mat_ids: &'a mut Vec<MaterialId>,
    pub entities: &'a mut Vec<Entity>,
}

impl<'a> NextCtx<'a> {
    #[inline] pub fn set_mat_id(&mut self, x: usize, y: usize, material_id: MaterialId) {
        self.cell_mat_ids[index(self.w, x, y)] = material_id;
    }

    #[inline] pub fn get_mat_id(&mut self, x: usize, y: usize) -> MaterialId {
        self.cell_mat_ids[index(self.w, x, y)]
    }
}



// -------------------------------- UTILITIES --------------------------------
/// Convert a 2D index to 1D.
#[inline] fn index(w: usize, x: usize, y: usize) -> usize { y * w + x }

#[inline] fn contains(w: usize, h: usize, x: usize, y: usize) -> bool {
    x < w && y < h
}