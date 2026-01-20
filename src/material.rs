use std::collections::HashMap;
use std::fs;
use anyhow::Result;
use macroquad::color::Color;
use ron::de::from_str;
use serde::Deserialize;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct MaterialId(pub u16);

#[derive(Deserialize, Default, Clone, Debug)]
#[serde(default)]
pub struct Material {
    #[serde(skip)]
    pub name: String,
    #[serde(skip)]
    pub color: Color,
    pub color_raw: (u8, u8, u8, u8),
    pub diffusivity: f32,

    #[serde(skip)]
    pub transform_cold_mat_id: Option<MaterialId>,
    pub transform_cold_mat_name: String,
    pub transform_cold_temp: f32,

    #[serde(skip)]
    pub transform_hot_mat_id: Option<MaterialId>,
    pub transform_hot_mat_name: String,
    pub transform_hot_temp: f32,
}

pub struct MaterialDb {
    defs: Vec<Material>,
    by_name: HashMap<String, MaterialId>,

    /// Diffusivity indexed by material ID, packed for cache locality during conductance updates.
    diffusivity_lookup: Box<[f32]>,
}

impl MaterialDb {
    pub fn new() -> Self {
        Self {
            defs: vec![],
            by_name: HashMap::new(),
            diffusivity_lookup: Box::default(),
        }
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

    #[inline(always)]
    pub fn get_diffusivity_lookup(&self) -> &[f32] {
        &self.diffusivity_lookup
    }

    #[inline(always)]
    pub fn diffusivity_of(&self, id: MaterialId) -> f32 {
        self.diffusivity_lookup[id.0 as usize]
    }

    pub fn load_ron_file(&mut self, path: &str) -> Result<()> {
        let text = fs::read_to_string(path)?;
        let mut map: HashMap<String, Material> = from_str(&text)?;

        // Build defs from loaded string.
        for (name, mut mat) in map.drain() {
            mat.name = name.clone(); // Populate the skipped field.
            mat.color = Color::from_rgba(mat.color_raw.0, mat.color_raw.1, mat.color_raw.2, mat.color_raw.3);
            mat.diffusivity = mat.diffusivity.clamp(0.0, 0.25);
            self.insert(mat);
        }

        // Build diffusivity lookup.
        self.diffusivity_lookup = self.defs.iter().map(|m| m.diffusivity).collect::<Box<[f32]>>();

        // Get material IDs for transforms.
        let ids: Vec<(Option<MaterialId>, Option<MaterialId>)> = self.defs.iter()
            .map(|m| {
                let cold = self.get_id(&m.transform_cold_mat_name);
                let hot  = self.get_id(&m.transform_hot_mat_name);
                (cold, hot)
            })
            .collect();

        // Assign material IDs for transforms. (Two passes due to borrow checker.)
        for (mat, (cold, hot)) in self.defs.iter_mut().zip(ids) {
            mat.transform_cold_mat_id = cold;
            mat.transform_hot_mat_id  = hot;
        }

        Ok(())
    }
}
