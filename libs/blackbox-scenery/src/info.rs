//! `SceneryInfo`: which model a scenery object uses.

use crate::layout::InfoLayout;

#[derive(Debug, Clone, PartialEq)]
pub struct SceneryInfo {
    pub name: String,
    /// `bStringHash` of the solid for each LOD, highest detail first; 0 = none.
    pub solid_keys: Vec<u32>,
    pub radius: f32,
    /// Non-zero for multi-part (hierarchical) models.
    pub hierarchy_hash: u32,
}

impl SceneryInfo {
    pub(crate) fn decode(r: &[u8], l: &InfoLayout) -> Self {
        let u32_at = |o: usize| u32::from_le_bytes(r[o..o + 4].try_into().unwrap());
        let name = &r[l.name..l.name + l.name_len];
        let name_len = name.iter().position(|&b| b == 0).unwrap_or(name.len());
        Self {
            name: String::from_utf8_lossy(&name[..name_len]).into_owned(),
            solid_keys: (0..l.lods).map(|i| u32_at(l.solid_keys + i * 4)).collect(),
            radius: f32::from_le_bytes(r[l.radius..l.radius + 4].try_into().unwrap()),
            hierarchy_hash: u32_at(l.hierarchy_hash),
        }
    }

    /// The most detailed solid, if any.
    pub fn best_solid(&self) -> Option<u32> {
        self.solid_keys.iter().copied().find(|&k| k != 0)
    }
}
