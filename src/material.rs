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

pub struct MaterialDB {
    defs: Vec<Option<Material>>,
    by_name: HashMap<String, MaterialId>,
    unused_ids: Vec<u16>,
}

impl MaterialDB {
    pub fn new() -> Self {
        Self { defs: vec![], by_name: HashMap::new(), unused_ids: vec![] }
    }

    fn get_next_id(&mut self) -> MaterialId {
        if let Some(id) = self.unused_ids.pop() {
            MaterialId(id)
        }
        else {
            let id = self.defs.len() as u16;
            self.defs.push(None);
            MaterialId(id)
        }
    }

    /// Insert or update material
    pub fn upsert(&mut self, m: Material) -> MaterialId {
        let name = m.name.clone();
        if let Some(&id) = self.by_name.get(&name) {
            self.defs[id.0 as usize] = Some(m);
            id
        } else {
            let id = self.get_next_id();
            self.by_name.insert(name, id);
            self.defs[id.0 as usize] = Some(m);
            id
        }
    }

    pub fn remove_by_name(&mut self, name: &str) {
        if let Some(id) = self.by_name.remove(name) {
            self.defs[id.0 as usize] = None;
            self.unused_ids.push(id.0);
        }
    }

    pub fn get_id(&self, name: &str) -> Option<MaterialId> {
        self.by_name.get(name).copied()
    }

    pub fn get(&self, id: &MaterialId) -> Option<&Material> {
        self.defs[id.0 as usize].as_ref()
    }

    pub fn get_mat_count(&self) -> usize { self.defs.len() }

    pub fn load_ron_file(&mut self, path: &str, purge_missing: bool) -> Result<()> {
        let text = fs::read_to_string(path)?;
        let mut map: HashMap<String, Material> = from_str(&text)?;

        let mut seen = std::collections::HashSet::new();
        for (name, mut mat) in map.drain() {
            mat.name = name.clone(); // populate the skipped field
            mat.color = Color::from_rgba(mat.color_raw.0, mat.color_raw.1, mat.color_raw.2, mat.color_raw.3);
            seen.insert(name.clone());
            self.upsert(mat);
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
