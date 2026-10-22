//! Plane triggers: the collections that inherit `planetrigger`. The ones that matter are the `resetplayertrigger`
//! family (`wrongway`, `resetplayertrigger1`): a flat zone across the road that sends the player back to the grid.
//! Spec: `docs/specs/race-wrongway.md`.

use blackbox_attrib::{CollectionRef, Database};
use glam::Vec3;

use super::{inherits, instances, name_of, placement};

/// A plane trigger: a thin slab across the road. The game builds it as a box with the width `Width` across, 50 m of
/// height on each side of its position and 1 m of thickness (`docs/specs/race-wrongway.md` section 2).
#[derive(Debug, Clone, PartialEq)]
pub struct PlaneTrigger {
    /// The collection's key (the hash of its name).
    pub key: u32,
    /// For example `race_bin_10/10_2_1_sprint/wrongway`: the race's name, a slash and the trigger's own.
    pub name: String,
    /// `Position` in physics space (the slab's centre, [`super::to_world`]).
    pub position: Vec3,
    /// `Rotation` in degrees, as a box trigger's (the slab's thin side faces [`super::world_heading`]).
    pub heading_deg: f32,
    /// `Width`, metres across the road: 150 on the template of `resetplayertrigger`, 200 on one trigger, 50 on a bare
    /// `planetrigger`.
    pub width: f32,
    /// `ResetsPlayer`: the player entering it goes back to the start of the race.
    pub resets_player: bool,
}

impl PlaneTrigger {
    /// Whether this trigger is a child of the race `race` (a full collection name): its name is the race's, a slash and
    /// one more name.
    pub fn is_child_of(&self, race: &str) -> bool {
        let Some(rest) = self.name.strip_prefix(race) else { return false };
        let Some(leaf) = rest.strip_prefix('/') else { return false };
        !leaf.is_empty() && !leaf.contains('/')
    }
}

/// Every non-template plane trigger, in database order.
pub(crate) fn planes(db: &Database) -> Vec<PlaneTrigger> {
    instances(db).filter(|c| inherits(*c, "planetrigger")).map(plane).collect()
}

fn plane(c: CollectionRef<'_>) -> PlaneTrigger {
    let p = placement(c);
    PlaneTrigger {
        key: c.key(),
        name: name_of(c),
        position: p.position,
        heading_deg: p.heading_deg,
        width: c.get_f32("Width").unwrap_or(0.0),
        resets_player: c.get_bool("ResetsPlayer").unwrap_or(false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plane_named(name: &str) -> PlaneTrigger {
        PlaneTrigger {
            key: 1,
            name: name.to_owned(),
            position: Vec3::ZERO,
            heading_deg: 0.0,
            width: 150.0,
            resets_player: true,
        }
    }

    #[test]
    fn a_trigger_is_a_child_of_the_race_that_prefixes_its_name() {
        let wrong_way = plane_named("race_bin_10/10_2_1_sprint/wrongway");
        assert!(wrong_way.is_child_of("race_bin_10/10_2_1_sprint"));
        assert!(!wrong_way.is_child_of("race_bin_10/10_2_1"), "a longer race name is not a prefix of its own");
        assert!(!wrong_way.is_child_of("race_bin_10/10_2_2_sprint"));
        assert!(!wrong_way.is_child_of("race_bin_10"), "a grandchild is not a child");
    }

    #[test]
    fn a_bare_name_is_nobodys_child() {
        assert!(!plane_named("resetplayertrigger").is_child_of(""));
        assert!(!plane_named("16_2_2_sprint/").is_child_of("16_2_2_sprint"));
    }
}
