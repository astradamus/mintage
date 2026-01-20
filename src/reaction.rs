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

            // Panic if any material reference is invalid.
            if material_db.get_id(&react_ref.in_a).is_none() {
                panic!("Invalid material reference: Reaction '{}' references missing in_a material '{}'",
                       name, react_ref.in_a);
            }
            if material_db.get_id(&react_ref.in_b).is_none() {
                panic!("Invalid material reference: Reaction '{}' references missing in_b material '{}'",
                       name, react_ref.in_b);
            }
            if material_db.get_id(&react_ref.out_a).is_none() {
                panic!("Invalid material reference: Reaction '{}' references missing out_a material '{}'",
                       name, react_ref.out_a);
            }
            if material_db.get_id(&react_ref.out_b).is_none() {
                panic!("Invalid material reference: Reaction '{}' references missing out_b material '{}'",
                       name, react_ref.out_b);
            }

            // Do not save if rate is zero or negative. Zero-rate reactions will
            // never occur, so there's no sense wasting time checking for them.
            if (react_ref.rate <= 0.0) { continue; }

            // Reaction validated, add to db.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_ron_file() {
        let mut mat_db = MaterialDb::new();
        mat_db.load_ron_file("assets_test/materials_test.ron").unwrap();

        let mut react_db = ReactionDb::new();;
        react_db.load_ron_file(&mat_db, "assets_test/reactions_test.ron").unwrap();

        assert_eq!(react_db.total_material_count, 12); // Ensure we got the right number of materials.

        {
            let mat_id_plant = mat_db.get_id("base:plant").unwrap();
            let mat_id_water = mat_db.get_id("base:water").unwrap();

            let react_id_plant_growth = react_db.by_name.get("base:plant+water=plant+plant").unwrap();
            let react_plant_growth = react_db.get(*react_id_plant_growth).unwrap();

            // Ensure reactions link to materials correctly.
            assert_eq!(react_plant_growth.in_a, mat_id_plant);
            assert_eq!(react_plant_growth.in_b, mat_id_water);
            assert_eq!(react_plant_growth.out_a, mat_id_plant);
            assert_eq!(react_plant_growth.out_b, mat_id_plant);

            // Ensure reactions are saved to the lookup table in both directions.
            assert_eq!(react_db.get_reaction_by_mats(mat_id_plant, mat_id_water), Some(*react_id_plant_growth));
            assert_eq!(react_db.get_reaction_by_mats(mat_id_water, mat_id_plant), Some(*react_id_plant_growth));
        }
    }

    #[test]
    #[should_panic(expected = "Invalid material reference")]
    fn test_invalid_ron_safety_in_a() {
        let mut mat_db = MaterialDb::new();
        mat_db.load_ron_file("assets_test/materials_test.ron").unwrap();
        let mut react_db = ReactionDb::new();

        // Ensure panic when invalid in_a material is referenced.
        react_db.load_ron_file(&mat_db, "assets_test/reactions_test_invalid_in_a.ron").unwrap();
    }

    #[test]
    #[should_panic(expected = "Invalid material reference")]
    fn test_invalid_ron_safety_in_b() {
        let mut mat_db = MaterialDb::new();
        mat_db.load_ron_file("assets_test/materials_test.ron").unwrap();
        let mut react_db = ReactionDb::new();

        // Ensure panic when invalid in_b material is referenced.
        react_db.load_ron_file(&mat_db, "assets_test/reactions_test_invalid_in_b.ron").unwrap();
    }

    #[test]
    #[should_panic(expected = "Invalid material reference")]
    fn test_invalid_ron_safety_out_a() {
        let mut mat_db = MaterialDb::new();
        mat_db.load_ron_file("assets_test/materials_test.ron").unwrap();
        let mut react_db = ReactionDb::new();

        // Ensure panic when invalid out_a material is referenced.
        react_db.load_ron_file(&mat_db, "assets_test/reactions_test_invalid_out_a.ron").unwrap();
    }

    #[test]
    #[should_panic(expected = "Invalid material reference")]
    fn test_invalid_ron_safety_out_b() {
        let mut mat_db = MaterialDb::new();
        mat_db.load_ron_file("assets_test/materials_test.ron").unwrap();
        let mut react_db = ReactionDb::new();

        // Ensure panic when invalid out_b material is referenced.
        react_db.load_ron_file(&mat_db, "assets_test/reactions_test_invalid_out_b.ron").unwrap();
    }

    #[test]
    fn test_ensure_db_starts_empty() {
        let react_db = ReactionDb::new();
        let count = react_db.defs.len();
        assert_eq!(count, 0);
        assert_eq!(react_db.total_material_count, 0);
    }

    #[test]
    fn test_get_invalid_id_returns_none() {
        let react_db = ReactionDb::new();
        let react_id = react_db.get_id("InvalidReact");
        assert_eq!(react_id, None);
    }
}
