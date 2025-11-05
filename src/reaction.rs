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

impl Reaction {
    pub fn has_in(&self, mat: MaterialId) -> bool {
        self.in_a == mat || self.in_b == mat
    }
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
    defs: Vec<Option<Reaction>>,
    by_name: HashMap<String, ReactionId>,
    unused_ids: Vec<u16>,
    total_material_count: usize,
    lookup: Vec<Option<ReactionId>>,
}

impl ReactionDb {
    pub fn new() -> Self {
        Self {
            defs: vec![],
            by_name: HashMap::new(),
            unused_ids: vec![],
            total_material_count: 0,
            lookup: vec![],
        }
    }

    fn get_next_id(&mut self) -> ReactionId {
        if let Some(id) = self.unused_ids.pop() {
            ReactionId(id)
        }
        else {
            let id = self.defs.len() as u16;
            self.defs.push(None);
            ReactionId(id)
        }
    }

    pub fn calc_lookup_index(&self, a: MaterialId, b: MaterialId)-> usize {
        (a.0 as usize * self.total_material_count) + (b.0 as usize)
    }
    pub fn get_reaction_by_mats(&self, a: MaterialId, b: MaterialId) -> Option<ReactionId> {
        self.lookup[self.calc_lookup_index(a, b)]
    }

    /// Insert or update reaction
    pub fn upsert(&mut self, m: Reaction) -> ReactionId {
        let name = m.name.clone();

        // If reaction with this name already exists, update the entry.
        if let Some(&id) = self.by_name.get(&name) {
            self.defs[id.0 as usize] = Some(m);
            id
        } else {
            // If there is a reaction with same materials, but different name, panic.
            if let Some(id) = self.get_reaction_by_mats(m.in_a, m.in_b) {
                let option = &self.defs[id.0 as usize];
                panic!("tried to insert reaction {}, but there is already a reaction using these two materials ({})",
                       name,
                       option.clone().unwrap().name,);
            }
            // If there is no reaction with same materials or name, insert new reaction.
            else {
                let id = self.get_next_id();
                self.by_name.insert(name, id);
                let index_a = self.calc_lookup_index(m.in_a, m.in_b);
                let index_b = self.calc_lookup_index(m.in_b, m.in_a);
                self.lookup[index_a] = Some(id);
                self.lookup[index_b] = Some(id);
                self.defs[id.0 as usize] = Some(m);
                id
            }
        }
    }

    pub fn remove_by_name(&mut self, name: &str) {
        if let Some(id) = self.by_name.remove(name) {
            self.defs[id.0 as usize] = None;
            self.unused_ids.push(id.0);
        }
    }

    pub fn get_id(&self, name: &str) -> Option<ReactionId> {
        self.by_name.get(name).copied()
    }

    pub fn get(&self, id: ReactionId) -> Option<&Reaction> {
        self.defs[id.0 as usize].as_ref()
    }

    pub fn load_ron_file(&mut self, material_db: &MaterialDb, path: &str, purge_missing: bool) -> Result<()> {

        // Setup from MaterialDB
        self.total_material_count = material_db.get_mat_count();
        self.lookup = vec![None; self.total_material_count * self.total_material_count];

        // Load from file
        let text = fs::read_to_string(path)?;
        let mut map: HashMap<String, ReactionRef> = from_str(&text)?;

        let mut seen = std::collections::HashSet::new();
        for (name, react_ref) in map.drain() {
            let react = Reaction {
                name,
                in_a: material_db.get_id(&react_ref.in_a).unwrap(),
                in_b: material_db.get_id(&react_ref.in_b).unwrap(),
                out_a: material_db.get_id(&react_ref.out_a).unwrap(),
                out_b: material_db.get_id(&react_ref.out_b).unwrap(),
                rate: react_ref.rate,
            };
            seen.insert(react.name.clone());
            self.upsert(react);
        }

        if purge_missing {
            let existing: Vec<String> = self.by_name.keys().cloned().collect();
            for name in existing {
                if !seen.contains(&name) {
                    self.remove_by_name(&name);
                }
            }
        }

        Ok(())
    }
}
