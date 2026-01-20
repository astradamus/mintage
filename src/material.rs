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

    /// Insert a new material. Returns the ID of the material.
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

            // Panic if transform reference is invalid (non-empty name but material not found).
            if !mat.transform_cold_mat_name.is_empty() && cold.is_none() {
                panic!("Invalid material reference: Material '{}' references missing cold transform material '{}'",
                       mat.name, mat.transform_cold_mat_name);
            }
            if !mat.transform_hot_mat_name.is_empty() && hot.is_none() {
                panic!("Invalid material reference: Material '{}' references missing hot transform material '{}'",
                       mat.name, mat.transform_hot_mat_name);
            }

            // Panic if both transforms are specified, but hot temp is not higher than cold temp.
            if !mat.transform_cold_mat_name.is_empty() && !mat.transform_hot_mat_name.is_empty() && mat.transform_cold_temp >= mat.transform_hot_temp {
                panic!("Invalid material configuration: Material '{}' has hot transform temperature ({}) equal to or lower than cold transform temperature ({})",
                       mat.name, mat.transform_hot_temp, mat.transform_cold_temp);
            }

            mat.transform_cold_mat_id = cold;
            mat.transform_hot_mat_id  = hot;
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

        assert_eq!(mat_db.get_mat_count(), 12); // Ensure all materials in test file are loaded.

        {
            let mat_id_diamond = mat_db.get_id("base:diamond").unwrap();
            let mat_diamond = mat_db.get(mat_id_diamond).unwrap();
            let diamond_diff = mat_diamond.diffusivity;

            // Ensure diffusivity is clamped. In test file, Diamond is set to 1.0, which must
            // be clamped to 0.25 to prevent terrible oscillations and other bugs.
            assert_eq!(diamond_diff, 0.25);

            // Ensure diffusivity lookup has same clamped value.
            assert_eq!(mat_db.diffusivity_of(mat_id_diamond), 0.25);
        }

        {
            let mat_id_insulation = mat_db.get_id("base:insulation").unwrap();
            let mat_insulation = mat_db.get(mat_id_insulation).unwrap();
            let insulation_diff = mat_insulation.diffusivity;

            // Ensure diffusivity is clamped. In test file, Insulation is set to -1.0,
            // which must be clamped to 0.0 to prevent bugs.
            assert_eq!(insulation_diff, 0.0);

            // Ensure diffusivity lookup has same clamped value.
            assert_eq!(mat_db.diffusivity_of(mat_id_insulation), 0.0);
        }

        {
            let mat_id_water = mat_db.get_id("base:water").unwrap();
            let mat_water = mat_db.get(mat_id_water).unwrap();
            let mat_id_steam = mat_db.get_id("base:steam").unwrap();
            let mat_steam = mat_db.get(mat_id_steam).unwrap();

            // Ensure transforms link to materials correctly.
            assert_eq!(mat_water.transform_hot_mat_id, Some(mat_id_steam));
            assert_eq!(mat_steam.transform_cold_mat_id, Some(mat_id_water));
        }
    }

    #[test]
    #[should_panic(expected = "Invalid material reference")]
    fn test_invalid_ron_safety_hot_transform() {
        let mut mat_db = MaterialDb::new();

        // Ensure panic when invalid hot transform material is referenced.
        mat_db.load_ron_file("assets_test/materials_test_invalid_hot.ron").unwrap();
    }

    #[test]
    #[should_panic(expected = "Invalid material reference")]
    fn test_invalid_ron_safety_cold_transform() {
        let mut mat_db = MaterialDb::new();

        // Ensure panic when invalid cold transform material is referenced.
        mat_db.load_ron_file("assets_test/materials_test_invalid_cold.ron").unwrap();
    }

    #[test]
    #[should_panic(expected = "Invalid material configuration")]
    fn test_invalid_ron_safety_transform_temps() {
        let mut mat_db = MaterialDb::new();

        // Ensure panic when a material specifies both hot and cold transforms, but hot
        // transform temperature is equal to or lower than cold transform temperature.
        mat_db.load_ron_file("assets_test/materials_test_invalid_transform_temps.ron").unwrap();
    }

    #[test]
    fn test_ensure_db_starts_empty() {
        let mat_db = MaterialDb::new();
        let count = mat_db.get_mat_count();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_get_invalid_id_returns_none() {
        let mat_db = MaterialDb::new();
        let material_id = mat_db.get_id("InvalidMat");
        assert_eq!(material_id, None);
    }
}
