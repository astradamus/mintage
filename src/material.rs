use std::collections::HashMap;
use std::fs;
use anyhow::Result;
use macroquad::color::Color;
use ron::de::from_str;
use serde::Deserialize;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct MaterialId(pub u16);

#[derive(Deserialize, Clone, Debug)]
pub struct Material {
    #[serde(skip)]
    pub name: String,
    #[serde(skip)]
    pub color: Color,
    pub color_raw: (u8, u8, u8, u8),
}

pub struct MaterialDb {
    defs: Vec<Material>,
    by_name: HashMap<String, MaterialId>,
}

impl MaterialDb {
    pub fn new() -> Self {
        Self { defs: vec![], by_name: HashMap::new() }
    }

    /// Insert a new material.
    fn insert(&mut self, m: Material) -> MaterialId {
        let name = m.name.clone();
        let id = MaterialId(self.defs.len() as u16);
        self.by_name.insert(name, id);
        self.defs.push(m);
        id
    }

    pub fn get_id(&self, name: &str) -> Option<MaterialId> {
        self.by_name.get(name).copied()
    }

    pub fn get(&self, id: MaterialId) -> Option<&Material> {
        self.defs.get(id.0 as usize)
    }

    pub fn get_mat_count(&self) -> usize { self.defs.len() }

    pub fn load_ron_file(&mut self, path: &str) -> Result<()> {
        let text = fs::read_to_string(path)?;
        let mut map: HashMap<String, Material> = from_str(&text)?;

        for (name, mut mat) in map.drain() {
            mat.name = name.clone(); // Populate the skipped field.
            mat.color = Color::from_rgba(mat.color_raw.0, mat.color_raw.1, mat.color_raw.2, mat.color_raw.3);
            self.insert(mat);
        }

        Ok(())
    }
}
