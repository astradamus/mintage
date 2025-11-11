use std::collections::HashMap;
use std::fs;
use anyhow::Result;
use ron::de::from_str;
use serde::Deserialize;
use crate::material::{MaterialDb, MaterialId};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ReactionId(pub u16);

#[derive(Clone, Debug)]
pub struct Reaction {
    pub name: String,
    pub in_a: MaterialId,
    pub in_b: MaterialId,
    pub out_a: MaterialId,
    pub out_b: MaterialId,
    pub rate: f32,
}

#[derive(Deserialize, Clone, Debug)]
pub struct ReactionRef {
    #[serde(skip)]
    pub name: String,
    pub in_a: String,
    pub in_b: String,
    pub out_a: String,
    pub out_b: String,
    pub rate: f32,
}

pub struct ReactionDb {
    defs: Vec<Reaction>,
    by_name: HashMap<String, ReactionId>,
    total_material_count: usize,
    lookup: Vec<Option<ReactionId>>,
}

impl ReactionDb {
    pub fn new() -> Self {
        Self {
            defs: vec![],
            by_name: HashMap::new(),
            total_material_count: 0,
            lookup: vec![],
        }
    }

    pub fn calc_lookup_index(&self, a: MaterialId, b: MaterialId) -> usize {
        (a.0 as usize * self.total_material_count) + (b.0 as usize)
    }

    pub fn get_reaction_by_mats(&self, a: MaterialId, b: MaterialId) -> Option<ReactionId> {
        self.lookup[self.calc_lookup_index(a, b)]
    }

    /// Insert a new reaction
    fn insert(&mut self, m: Reaction) -> ReactionId {
        let name = m.name.clone();

        // If there is a reaction with same materials, panic.
        if let Some(id) = self.get_reaction_by_mats(m.in_a, m.in_b) {
            let existing = &self.defs[id.0 as usize];
            panic!("tried to insert reaction {}, but there is already a reaction using these two materials ({})", name, existing.name);
        }

        let id = ReactionId(self.defs.len() as u16);
        self.by_name.insert(name, id);
        let index_a = self.calc_lookup_index(m.in_a, m.in_b);
        let index_b = self.calc_lookup_index(m.in_b, m.in_a);
        self.lookup[index_a] = Some(id);
        self.lookup[index_b] = Some(id);
        self.defs.push(m);
        id
    }

    pub fn get_id(&self, name: &str) -> Option<ReactionId> {
        self.by_name.get(name).copied()
    }

    pub fn get(&self, id: ReactionId) -> Option<&Reaction> {
        self.defs.get(id.0 as usize)
    }

    pub fn load_ron_file(&mut self, material_db: &MaterialDb, path: &str) -> Result<()> {
        // Setup from MaterialDB
        self.total_material_count = material_db.get_mat_count();
        self.lookup = vec![None; self.total_material_count * self.total_material_count];

        // Load from file
        let text = fs::read_to_string(path)?;
        let mut map: HashMap<String, ReactionRef> = from_str(&text)?;

        for (name, react_ref) in map.drain() {
            let react = Reaction {
                name,
                in_a: material_db.get_id(&react_ref.in_a).unwrap(),
                in_b: material_db.get_id(&react_ref.in_b).unwrap(),
                out_a: material_db.get_id(&react_ref.out_a).unwrap(),
                out_b: material_db.get_id(&react_ref.out_b).unwrap(),
                rate: react_ref.rate,
            };
            self.insert(react);
        }

        Ok(())
    }
}
