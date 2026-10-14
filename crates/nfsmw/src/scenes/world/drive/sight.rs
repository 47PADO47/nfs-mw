//! The cops' line of sight (`docs/specs/ai-pursuit-cop-cars.md` section 2): a ray from a cop's eyes to the player's
//! that the world's geometry blocks. This is a separate query from the body's wall probe (`walls.rs`), which ignores
//! floors and grouped scenery on purpose. Here road decks, bridge undersides and buildings all block, so a cop below a
//! deck or around a corner does not see the player.
//!
//! The one pass-through is a one-sided barrier met from its back side (`docs/formats/collision.md`, "Query semantics"
//! 6): a cop looking at the back of a guard rail sees through it.

use blackbox_collision::{BARRIER_TWO_SIDED, CollisionWorld, HitKind, RayOptions};
use glam::Vec3;

use super::walls::{Hit, HitInfo};

/// Whether a hit blocks the sight: any face does, and a barrier does unless it is one-sided and met from behind.
fn blocks(kind: HitKind, front_facing: bool, surface_flags: u8) -> bool {
    kind != HitKind::Barrier || front_facing || surface_flags & BARRIER_TWO_SIDED != 0
}

/// Ray casts for the cops' sight: the first blocking face or barrier on the segment, with no group exclusion.
pub fn sight_cast(collision: &CollisionWorld) -> impl Fn(Vec3, Vec3) -> Option<Hit> + '_ {
    let options = RayOptions { exclude: 0, skip_groups: false, ..RayOptions::default() };
    move |from, to| {
        let hit = collision.ray_cast_filtered(from.to_array(), to.to_array(), &options, |h| {
            blocks(h.kind, h.front_facing, h.surface_flags)
        })?;
        let info = HitInfo {
            barrier: hit.kind == HitKind::Barrier,
            surface: hit.surface_hash,
            section: hit.section,
            instance: hit.instance,
        };
        Some(Hit { point: Vec3::from(hit.point), normal: Vec3::from(hit.normal), prop: None, info: Some(info) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_road_deck_or_bridge_underside_blocks_the_sight() {
        assert!(blocks(HitKind::Face, true, 0), "a floor face is a face");
        assert!(blocks(HitKind::Face, false, 0), "seen from below too");
    }

    #[test]
    fn a_one_sided_barrier_passes_from_behind_only() {
        assert!(blocks(HitKind::Barrier, true, 0), "from the front it blocks");
        assert!(!blocks(HitKind::Barrier, false, 0), "from behind it passes");
        assert!(blocks(HitKind::Barrier, false, BARRIER_TWO_SIDED), "two-sided blocks from behind");
    }
}
