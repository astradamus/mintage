use std::mem;
use macroquad::logging::warn;
use crate::material::{MaterialId, Material, MaterialDB};
use crate::reaction::{ReactionDB};

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

#[derive(Copy, Clone, Debug)]
struct Entity {

}

impl Entity {
    fn empty() -> Self {
        Self { }
    }
}

pub struct World {
    pub w: usize,
    pub h: usize,

    pub mat_ids: DoubleBuffer<Vec<MaterialId>>,
    pub entities: DoubleBuffer<Vec<Entity>>,

    pub materials: MaterialDB,
    pub reactions: ReactionDB,
}

impl World {
    pub fn new(w: usize, h: usize) -> Self {
        let mat_ids = vec![MaterialId(0); w * h];
        let entities = vec![Entity::empty(); w * h];

        let mut material_db = MaterialDB::new();
        material_db
            .load_ron_file("assets/materials_base.ron", true)
            .expect("failed to load materials");

        let mut reaction_db = ReactionDB::new();
        (reaction_db)
            .load_ron_file(&material_db, "assets/reactions_base.ron", true)
            .expect("failed to load reactions");

        Self {
            w, h,
            mat_ids: DoubleBuffer::new(mat_ids),
            entities: DoubleBuffer::new(entities),
            materials: material_db,
            reactions: reaction_db,
        }
    }

    pub fn sync_all(&mut self) {
        self.mat_ids.sync();
        self.entities.sync();
    }

    pub fn swap_all(&mut self) {
        self.mat_ids.swap();
        self.entities.swap();
    }

    pub fn get_curr_mat_id_at(&self, x: usize, y: usize) -> Option<&MaterialId> {
        if let Some(cell) = self.mat_ids.cur.get(index(self.w, x, y)) {
            Some(cell)
        }
        else {
            warn!("tried get_curr_mat_id_at for out-of-bounds cell: ({x}, {y})");
            None
        }
    }

    pub fn get_curr_mat_at(&self, x: usize, y: usize) -> Option<&Material> {
        if let Some(id) = self.get_curr_mat_id_at(x, y) {
            self.materials.get(id)
        }
        else {
            None
        }
    }

    pub fn ctx_pair(&mut self) -> (CurrCtx<'_>, NextCtx<'_>) {
        let curr = CurrCtx {
            w: self.w,
            h: self.h,
            mat_ids: &self.mat_ids.cur,
            entities: &self.entities.cur,
            materials: &self.materials,
            reactions: &self.reactions,
        };
        let next = NextCtx {
            w: self.w,
            h: self.h,
            mat_ids: &mut self.mat_ids.next,
            entities: &mut self.entities.next,
        };
        (curr, next)
    }
}



// ------------------------------ CURR FRAME CONTEXT -------------------------------
pub struct CurrCtx<'a> {
    pub w: usize,
    pub h: usize,
    pub mat_ids: &'a [MaterialId],
    pub entities: &'a [Entity],
    pub materials: &'a MaterialDB,
    pub reactions: &'a ReactionDB,
}

impl<'a> CurrCtx<'a> {
    #[inline] pub fn get_mat_id(&self, x: usize, y: usize) -> MaterialId {
        self.mat_ids[index(self.w, x, y)]
    }

    pub fn contains(&self, x: isize, y: isize) -> bool {
        contains(self.w, self.h, x as usize, y as usize)
    }
}



// ------------------------------ NEXT FRAME CONTEXT ------------------------------

pub struct NextCtx<'a> {
    pub w: usize,
    pub h: usize,
    pub mat_ids: &'a mut Vec<MaterialId>,
    pub entities: &'a mut Vec<Entity>,
}

impl<'a> NextCtx<'a> {
    #[inline] pub fn set_mat_id(&mut self, x: usize, y: usize, material_id: MaterialId) {
        self.mat_ids[index(self.w, x, y)] = material_id;
    }

    #[inline] pub fn get_mat_id(&mut self, x: usize, y: usize) -> MaterialId {
        self.mat_ids[index(self.w, x, y)]
    }
}



// -------------------------------- UTILITIES --------------------------------
/// Convert a 2D index to 1D.
#[inline] fn index(w: usize, x: usize, y: usize) -> usize { y * w + x }

#[inline] fn contains(w: usize, h: usize, x: usize, y: usize) -> bool {
    x < w && y < h
}