//! `SceneryInstance`: one placed copy of a scenery object.

use crate::layout::InstanceLayout;

#[derive(Debug, Clone, PartialEq)]
pub struct SceneryInstance {
    /// World-space bounding box.
    pub bbox_min: [f32; 3],
    pub bbox_max: [f32; 3],
    pub exclude_flags: u32,
    pub preculler_index: i16,
    pub lighting_context: i16,
    pub position: [f32; 3],
    /// Rows are the object's x, y and z axes in world space (may include scale/mirroring).
    pub rotation: [[f32; 3]; 3],
    /// Index into the section's [`crate::SceneryInfo`] list.
    pub info_index: i16,
}

impl SceneryInstance {
    pub(crate) fn decode(r: &[u8], l: &InstanceLayout) -> Self {
        let f32_at = |o: usize| f32::from_le_bytes(r[o..o + 4].try_into().unwrap());
        let i16_at = |o: usize| i16::from_le_bytes([r[o], r[o + 1]]);
        let vec3 = |o: usize| [f32_at(o), f32_at(o + 4), f32_at(o + 8)];
        let rot = |row: usize, col: usize| f32::from(i16_at(l.rotation + (row * 3 + col) * 2)) / l.rotation_scale;
        Self {
            bbox_min: vec3(l.bbox_min),
            bbox_max: vec3(l.bbox_max),
            exclude_flags: u32::from_le_bytes(r[l.exclude_flags..l.exclude_flags + 4].try_into().unwrap()),
            preculler_index: i16_at(l.preculler_index),
            lighting_context: i16_at(l.lighting_context),
            position: vec3(l.position),
            rotation: [0, 1, 2].map(|row| [0, 1, 2].map(|col| rot(row, col))),
            info_index: i16_at(l.info_index),
        }
    }

    /// Whether the instance is drawn in a view with `view_flags`
    /// (`docs/specs/scenery-visibility.md`).
    pub fn visible_in(&self, view_flags: u32, rules: &crate::layout::VisibilityRules) -> bool {
        let exclude = (self.exclude_flags & 0xFF) ^ rules.inverted_bits;
        exclude & view_flags == 0
    }

    /// The object-to-world matrix as four columns (column-vector convention):
    /// `world = x * rotation[0] + y * rotation[1] + z * rotation[2] + position`.
    pub fn matrix_columns(&self) -> [[f32; 4]; 4] {
        let [r0, r1, r2] = self.rotation;
        let p = self.position;
        [[r0[0], r0[1], r0[2], 0.0], [r1[0], r1[1], r1[2], 0.0], [r2[0], r2[1], r2[2], 0.0], [p[0], p[1], p[2], 1.0]]
    }
}
