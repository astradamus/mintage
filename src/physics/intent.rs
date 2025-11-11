use crate::material::MaterialId;

#[derive(Debug, Clone, Copy)]
pub(crate) enum CellIntent {
    Transform {
        cell: (usize, usize),
        out: MaterialId,
    },
    Reaction {
        cell_a: (usize, usize),
        cell_b: (usize, usize),
        out_a: MaterialId,
        out_b: MaterialId,
    },
    Movement {
        from: (usize, usize),
        to: (usize, usize),
    },
}

impl CellIntent {
    pub(crate) fn affected_cells(&self) -> Vec<(usize, usize)> {
        match self {
            CellIntent::Transform { cell, ..} => {vec![*cell]},
            CellIntent::Reaction { cell_a, cell_b, .. } => vec![*cell_a, *cell_b],
            CellIntent::Movement { from, to} => {vec![*from, *to]},
        }
    }
}