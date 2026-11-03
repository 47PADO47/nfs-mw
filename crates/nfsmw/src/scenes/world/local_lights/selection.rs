//! Which recognized fixtures emit this frame: pure selection rules, kept apart from the scene scan.
use super::fixtures::Fixture;
use super::profile::{EMITTER, FADE_METRES, GLOW_BUDGET, GLOW_DISTANCE, UPRIGHT_COS};
use blackbox_gfx::{Instance, InstanceKey, LocalLight, MeshHandle};
use glam::{Mat4, Vec3};

/// A visible bulb: where it glows, its intensity, the lens half-axes and the lantern's vertical half-extent
/// (zero lens axes: no lens quad; non-zero lantern extent: the mesh glass glows instead). Same layout as
/// `WorldScene::street_glows`.
pub(in crate::scenes::world) type StreetGlow = (Vec3, f32, Vec3, Vec3, Vec3);

/// The frame's fixture switches, after day-night and strength are applied.
#[derive(Clone, Copy, Debug)]
pub(super) struct Switches {
    /// Tunnel and other non-street fixtures.
    pub lamps: bool,
    /// Street fixtures: the switch is on, it is night and the strength is non-zero.
    pub street: bool,
    pub street_night: f32,
    pub street_strength: f32,
    pub street_range: f32,
    pub lamp_strength: f32,
    pub street_glow: f32,
}

impl Switches {
    pub fn any(&self) -> bool {
        self.lamps || self.street
    }

    /// Glass emission of a powered street fixture.
    pub fn emission(&self) -> f32 {
        self.street_night * self.street_glow * self.street_strength.min(1.0)
    }
}

/// One fixture that may emit this frame.
#[derive(Clone, Copy, Debug)]
pub(super) struct Candidate {
    pub distance: f32,
    /// Stable identity for distance ties: the placement and the fixture within it.
    pub owner: InstanceKey,
    pub index: usize,
    pub street: bool,
    pub light: LocalLight,
    /// The bulb, with the lantern's vertical half-extent, which [`push_glows`] passes on only with `glass`.
    pub glow: StreetGlow,
    /// The drawn model's own glass emits.
    pub glass: bool,
    /// The placement's LOD meshes with emissive glass when only some LODs have it: `glass` then depends on which
    /// one the main view draws ([`resolve_drawn_glass`]).
    pub glass_meshes: [Option<MeshHandle>; 4],
}

/// Whether `fixture` may emit: its family is switched on, and a street fixture is powered and still upright.
/// `powered` is only asked for street fixtures.
pub(super) fn admits(fixture: &Fixture, switches: &Switches, powered: impl FnOnce() -> bool, up: Vec3) -> bool {
    if !fixture.street {
        return switches.lamps;
    }
    switches.street && up.normalize_or_zero().dot(Vec3::Z) >= UPRIGHT_COS && powered()
}

/// The light and bulb of `fixture` placed by `transform`, seen from `camera`.
pub(super) fn candidate(
    fixture: &Fixture,
    transform: Mat4,
    owner: InstanceKey,
    index: usize,
    camera: Vec3,
    switches: &Switches,
) -> Candidate {
    let position = transform.transform_point3(fixture.position);
    let (range, strength) = match fixture.street {
        true => (switches.street_range, switches.street_night * switches.street_strength),
        false => (EMITTER.fixture_range, switches.lamp_strength),
    };
    let lantern = fixture.lantern_half_height > 0.0;
    // A lantern's lens group is its whole head: without emissive glass only the camera-facing glow stands in.
    let (lens_x, lens_y) = match lantern {
        true => (Vec3::ZERO, Vec3::ZERO),
        false => (
            transform.transform_vector3(Vec3::X * fixture.lens_half.x),
            transform.transform_vector3(Vec3::Y * fixture.lens_half.y),
        ),
    };
    let lantern_z = transform.transform_vector3(Vec3::Z * fixture.lantern_half_height);
    Candidate {
        distance: position.distance(camera),
        owner,
        index,
        street: fixture.street,
        light: LocalLight {
            position: position.to_array(),
            direction: transform.transform_vector3(fixture.direction).normalize_or_zero().to_array(),
            casts_shadow: true,
            shadow_exclude: lantern.then_some(owner),
            upper_plane: None,
            range,
            outer_cos: EMITTER.outer_cos,
            inner_cos: EMITTER.inner_cos,
            color: (Vec3::from_array(EMITTER.color) * EMITTER.intensity * strength).to_array(),
        },
        glow: (transform.transform_point3(fixture.glow_position), 0.0, lens_x, lens_y, lantern_z),
        glass: fixture.glass,
        glass_meshes: [None; 4],
    }
}

/// Settle `glass` for the candidates whose LODs differ, from the meshes the main view `drawn` for their owner last
/// frame. An owner not drawn there has no visible glass. `owners` and `seen` are reusable scratch lists.
pub(super) fn resolve_drawn_glass(
    candidates: &mut [Candidate],
    drawn: &[Instance],
    owners: &mut Vec<InstanceKey>,
    seen: &mut Vec<(InstanceKey, MeshHandle)>,
) {
    owners.clear();
    seen.clear();
    let mixed = |c: &Candidate| c.glass_meshes.iter().any(Option::is_some);
    owners.extend(candidates.iter().filter(|c| mixed(c)).map(|c| c.owner));
    if owners.is_empty() {
        return;
    }
    owners.sort_unstable();
    owners.dedup();
    let listed = drawn.iter().filter(|i| owners.binary_search(&i.owner_key).is_ok());
    seen.extend(listed.map(|i| (i.owner_key, i.mesh)));
    seen.sort_unstable_by_key(|&(owner, _)| owner);
    for candidate in candidates.iter_mut().filter(|c| mixed(c)) {
        let start = seen.partition_point(|&(owner, _)| owner < candidate.owner);
        let meshes = seen[start..].iter().take_while(|&&(owner, _)| owner == candidate.owner);
        candidate.glass = meshes.into_iter().any(|(_, mesh)| candidate.glass_meshes.contains(&Some(*mesh)));
    }
}

/// Bulbs remain visible beyond the surface-light distance, up to this.
pub(super) fn glow_distance(light_distance: f32) -> f32 {
    light_distance.max(GLOW_DISTANCE)
}

/// Nearest first, then by stable identity.
pub(super) fn sort(candidates: &mut [Candidate]) {
    candidates
        .sort_by(|a, b| a.distance.total_cmp(&b.distance).then(a.owner.cmp(&b.owner)).then(a.index.cmp(&b.index)));
}

/// A smooth 0..1 fade over [`FADE_METRES`] inside `boundary`.
pub(super) fn fade(boundary: f32, distance: f32) -> f32 {
    let t = ((boundary - distance) / FADE_METRES).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Visible bulbs of the sorted street candidates, within their own budget so a missing light or shadow slot does
/// not switch an otherwise visible lamp off. They fade towards the budget or distance boundary.
pub(super) fn push_glows(sorted: &[Candidate], switches: &Switches, out: &mut Vec<StreetGlow>) {
    let mut street = sorted.iter().filter(|candidate| candidate.street);
    let boundary = street.clone().nth(GLOW_BUDGET).map_or(GLOW_DISTANCE, |c| c.distance.min(GLOW_DISTANCE));
    for candidate in street.by_ref().take(GLOW_BUDGET) {
        let (at, _, lens_x, lens_y, lantern_z) = candidate.glow;
        let lantern_z = if candidate.glass { lantern_z } else { Vec3::ZERO };
        let intensity = switches.street_night * fade(boundary, candidate.distance) * switches.street_strength.min(1.0);
        out.push((at, intensity, lens_x, lens_y, lantern_z));
    }
}

/// Surface-light slots left under `limit` after `used` lights.
pub(super) fn remaining_slots(limit: u32, used: usize) -> usize {
    (limit as usize).saturating_sub(used)
}

/// The nearest sorted candidates within `distance` that fit `remaining` slots, faded towards the boundary.
pub(super) fn push_lights(sorted: &[Candidate], distance: f32, remaining: usize, out: &mut Vec<LocalLight>) {
    let mut near = sorted.iter().take_while(|candidate| candidate.distance < distance);
    let boundary = near.clone().nth(remaining).map_or(distance, |c| c.distance);
    for candidate in near.by_ref().take(remaining) {
        let mut light = candidate.light;
        light.color = (Vec3::from_array(light.color) * fade(boundary, candidate.distance)).to_array();
        out.push(light);
    }
}

#[cfg(test)]
#[path = "selection_tests.rs"]
mod tests;
