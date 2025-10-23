use std::mem;
use macroquad::logging::warn;
use crate::material::{MaterialId, Material, MaterialDB};
use crate::physics::PhysicsModule;

/// Generic double buffer over any T. We use it for `Vec<Cell>` and `Vec<Entity>`.
#[derive(Debug)]
pub struct DoubleBuffer<T> {
    pub cur: T,
    pub next: T,
}

impl<T: Clone> DoubleBuffer<T> {
    pub fn new(initial: T) -> Self {
        Self { cur: initial.clone(), next: initial }
    }

    /// Copy current into next so modules can do deltas on top.
    pub fn sync(&mut self) {
        // For Vec<T>, this clones the elements; for POD-ish types it's very fast.
        self.next.clone_from(&self.cur);
    }

    pub fn swap(&mut self) {
        mem::swap(&mut self.cur, &mut self.next);
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Cell { pub mat_id: MaterialId }

impl Cell {
    fn empty() -> Self {
        Self { mat_id: MaterialId(0) }
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

    pub cells: DoubleBuffer<Vec<Cell>>,
    pub entities: DoubleBuffer<Vec<Entity>>,

    pub materials: MaterialDB,
}

impl World {
    pub fn new(w: usize, h: usize) -> Self {
        let mut cells = vec![Cell::empty(); w * h];
        let mut entities = vec![Entity::empty(); w * h];

        let mut material_db = MaterialDB::new();
        material_db
            .load_ron_file("assets/materials_base.ron", true)
            .expect("failed to load materials");

        Self {
            w, h,
            cells: DoubleBuffer::new(cells),
            entities: DoubleBuffer::new(entities),
            materials: material_db,
        }
    }

    pub fn sync_all(&mut self) {
        self.cells.sync();
        self.entities.sync();
    }

    pub fn swap_all(&mut self) {
        self.cells.swap();
        self.entities.swap();
    }

    pub fn mat_id_at(&self, x: usize, y: usize) -> Option<MaterialId> {
        if let Some(cell) = self.cells.cur.get(index(self.w, x, y)) {
            Some(cell.mat_id)
        }
        else {
            warn!("tried mat_id_at for out-of-bounds cell: ({x}, {y})");
            None
        }
    }

    pub fn mat_at(&self, x: usize, y: usize) -> Option<&Material> {
        if let Some(id) = self.mat_id_at(x, y) {
            self.materials.get(id)
        }
        else {
            None
        }
    }

    pub fn ctx_pair(&mut self) -> (ReadCtx<'_>, WriteCtx<'_>) {
        let read = ReadCtx {
            w: self.w,
            h: self.h,
            cells: &self.cells.cur,
            entities: &self.entities.cur,
            materials: &self.materials,
        };
        let write = WriteCtx {
            w: self.w,
            h: self.h,
            cells: &mut self.cells.next,
            entities: &mut self.entities.next,
        };
        (read, write)
    }
}



// ------------------------------ READ CONTEXT -------------------------------
pub struct ReadCtx<'a> {
    pub w: usize,
    pub h: usize,
    pub cells: &'a [Cell],
    pub entities: &'a [Entity],
    pub materials: &'a MaterialDB,
}

impl<'a> ReadCtx<'a> {
    #[inline] pub fn cell(&self, x: usize, y: usize) -> &Cell {
        &self.cells[index(self.w, x, y)]
    }

    pub fn try_cell(&self, x: isize, y: isize) -> Option<&Cell> {
        if !contains(self.w, self.h, x as usize, y as usize) { return None; }
        let i = index(self.w, x as usize, y as usize);
        Some(&self.cells[i])
    }

    pub fn contains(&self, x: isize, y: isize) -> bool {
        contains(self.w, self.h, x as usize, y as usize)
    }
}



// ------------------------------ WRITE CONTEXT ------------------------------

pub struct WriteCtx<'a> {
    pub w: usize,
    pub h: usize,
    pub cells: &'a mut Vec<Cell>,      // writing into NEXT cells
    pub entities: &'a mut Vec<Entity>, // writing into NEXT entities
}

impl<'a> WriteCtx<'a> {
    /// Get the cell at the given location, without bounds checking (unsafe!).
    #[inline] pub fn cell_mut(&mut self, x: usize, y: usize) -> &mut Cell {
        &mut self.cells[index(self.w, x, y)]
    }

    /// Try to get the cell at given location, or None if out of bounds.
    pub fn try_cell_mut(&mut self, x: isize, y: isize) -> Option<&mut Cell> {
        if !contains(self.w, self.h, x as usize, y as usize) { return None; }
        let i = index(self.w, x as usize, y as usize);
        Some(&mut self.cells[i])
    }
}



// -------------------------------- UTILITIES --------------------------------
/// Convert a 2D index to 1D.
#[inline] fn index(w: usize, x: usize, y: usize) -> usize { y * w + x }

#[inline] fn contains(w: usize, h: usize, x: usize, y: usize) -> bool {
    x >= 0 && y >= 0 && x < w && y < h
}