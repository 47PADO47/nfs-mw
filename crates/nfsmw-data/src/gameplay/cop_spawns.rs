//! The cop spawn points of the scripted races: the children `cop_spawn_1`, `cop_spawn_2` of a prologue race, where its
//! script places mid-size police cars when the player enters `cop_spawn_trigger` (`docs/specs/story-flow.md` section 2.4).
//! Each is a marker with a `Position` and a `Rotation`.

use blackbox_attrib::Database;

use super::{Placement, instances, name_of, placement};

/// The leaf name prefix of a spawn point.
const PREFIX: &str = "cop_spawn_";

/// One cop spawn point.
#[derive(Debug, Clone, PartialEq)]
pub struct CopSpawn {
    /// The collection's key (the hash of its full name).
    pub key: u32,
    /// The full name, for example `16_2_1_d_day_intro/cop_spawn_1`.
    pub name: String,
    /// `Position` in physics space and `Rotation` in degrees.
    pub placement: Placement,
}

impl CopSpawn {
    /// Whether this is a spawn point of the race `race` (a full collection name): its name is the race's, a slash and
    /// `cop_spawn_` with a number.
    pub fn is_of(&self, race: &str) -> bool {
        let Some(leaf) = self.name.strip_prefix(race).and_then(|rest| rest.strip_prefix('/')) else { return false };
        leaf.strip_prefix(PREFIX).is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
    }
}

/// Every cop spawn point of the database, sorted by name (so `cop_spawn_1` comes before `cop_spawn_2`).
pub(crate) fn cop_spawns(db: &Database) -> Vec<CopSpawn> {
    let mut out: Vec<CopSpawn> = instances(db)
        .filter_map(|c| {
            let name = name_of(c);
            let leaf = name.rsplit('/').next()?;
            leaf.starts_with(PREFIX).then(|| CopSpawn { key: c.key(), name: name.clone(), placement: placement(c) })
        })
        .collect();
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec3;

    fn spawn(name: &str) -> CopSpawn {
        CopSpawn { key: 1, name: name.into(), placement: Placement { position: Vec3::ZERO, heading_deg: 0.0 } }
    }

    #[test]
    fn a_spawn_belongs_to_the_race_it_is_a_child_of() {
        let one = spawn("16_2_1_d_day_intro/cop_spawn_1");
        assert!(one.is_of("16_2_1_d_day_intro"));
        assert!(!one.is_of("16_2_2_sprint"));
        assert!(!spawn("16_2_1_d_day_intro/cop_spawn_trigger").is_of("16_2_1_d_day_intro"), "the trigger is no spawn");
        assert!(!spawn("16_2_1_d_day_intro/x/cop_spawn_1").is_of("16_2_1_d_day_intro"), "only direct children");
    }
}
