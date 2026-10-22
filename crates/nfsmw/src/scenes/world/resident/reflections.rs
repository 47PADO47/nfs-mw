//! Separate authored road-view placement; shared GPU resources keep the models alive.
use super::{Placed, SKIPPED_MODELS, SectionResources};
use blackbox_gfx::{InstanceKey, MeshHandle};
use blackbox_scene::Aabb;
use glam::Mat4;
use nfsmw_data::world::SectionData;

/// The source road queue's stable view flags; optional race/event bits are separate.
pub const ROAD_VIEW_FLAGS: u32 = 0x0800_0850;

#[derive(Clone)]
pub struct ReflectionPlaced {
    pub lods: [Option<MeshHandle>; 4],
    pub transform: Mat4,
    pub bounds: Aabb,
    pub radius: f32,
    pub flags: u32,
    pub prop_id: u32,
    pub key: InstanceKey,
    /// Matching normal-view placement, if this source is visible in both views.
    pub player_index: Option<usize>,
}

impl ReflectionPlaced {
    pub fn bind_player(&mut self, player: &[Placed], fallback: InstanceKey) {
        self.key = fallback;
        if let Some(p) = self.player_index.and_then(|i| player.get(i)) {
            self.prop_id = p.prop_id;
            self.key = p.key;
        }
    }
}

pub fn visible(flags: u32, view_flags: u32) -> bool {
    let expanded = match flags & 0x0800_0040 != 0 {
        true => flags | 0x0800_0040,
        false => flags,
    };
    (expanded ^ 0xFFFF_FF60) & view_flags & 0x0800_00FF == 0
}

/// One instance of the authored road view, without its meshes (see [`super::prepare`]).
pub struct PreparedReflection {
    /// The solid of each LOD slot, 0 for none.
    keys: [u32; 4],
    transform: Mat4,
    bounds: Aabb,
    radius: f32,
    flags: u32,
    /// The player view draws it too.
    player_visible: bool,
    /// The road view draws it.
    road_visible: bool,
}

#[cfg(test)]
pub fn place_reflections(
    data: &SectionData,
    own: &SectionResources,
    shared: &SectionResources,
) -> Vec<ReflectionPlaced> {
    resolve_reflections(&prepare_reflections(data), own, shared)
}

/// The renderer-free half of [`place_reflections`]: every instance of a model that is drawn at all.
pub fn prepare_reflections(data: &SectionData) -> Vec<PreparedReflection> {
    let rules = &blackbox_scenery::layout::MOST_WANTED.visibility;
    let mut out = Vec::new();
    for section in &data.scenery {
        for inst in &section.instances {
            let Some(info) = section.info_of(inst) else { continue };
            if SKIPPED_MODELS.contains(&info.name.as_str()) {
                continue;
            }
            out.push(PreparedReflection {
                keys: std::array::from_fn(|slot| info.solid_keys.get(slot).copied().unwrap_or(0)),
                transform: Mat4::from_cols_array_2d(&inst.matrix_columns()),
                bounds: Aabb::new(inst.bbox_min, inst.bbox_max),
                radius: info.radius,
                flags: inst.exclude_flags,
                player_visible: inst.visible_in(rules.player_view, rules),
                road_visible: visible(inst.exclude_flags, ROAD_VIEW_FLAGS),
            });
        }
    }
    out
}

/// The other half: joins the prepared instances to the meshes in `own` and `shared`. An instance without a mesh is
/// dropped, and does not take a place in the player view's numbering.
pub fn resolve_reflections(
    prepared: &[PreparedReflection],
    own: &SectionResources,
    shared: &SectionResources,
) -> Vec<ReflectionPlaced> {
    let mut player_index = 0;
    let mut out = Vec::new();
    for item in prepared {
        let lods = std::array::from_fn(|slot| {
            Some(item.keys[slot]).filter(|&hash| hash != 0).and_then(|hash| own.mesh(hash, shared)).map(|m| m.0)
        });
        if lods.iter().all(Option::is_none) {
            continue;
        }
        let source_player = item.player_visible.then_some(player_index);
        if source_player.is_some() {
            player_index += 1;
        }
        if !item.road_visible {
            continue;
        }
        out.push(ReflectionPlaced {
            lods,
            transform: item.transform,
            bounds: item.bounds,
            radius: item.radius,
            flags: item.flags,
            prop_id: 0,
            key: InstanceKey::TRANSIENT,
            player_index: source_player,
        });
    }
    out
}

#[cfg(test)]
mod tests;
