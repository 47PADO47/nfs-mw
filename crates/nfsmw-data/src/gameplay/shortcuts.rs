//! The shortcut markers of the races (class `shortcut`, parent `marker`): a place a racer may cut through, with the
//! chance it does. Layout: `docs/formats/gameplay-races.md` section 3.4.

use blackbox_attrib::{CollectionRef, Database};
use glam::Vec3;

use super::{inherits, instances, placement};

/// One shortcut marker.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShortcutMarker {
    /// The collection's key (the hash of its name); a race's `Shortcuts` lists it.
    pub key: u32,
    /// `Position` in physics space.
    pub position: Vec3,
    /// `Rotation` in degrees; the heading is [`super::world_heading`].
    pub heading_deg: f32,
    /// `ShortcutMinChance` and `ShortcutMaxChance` as stored: 0 to 1, or percentages when above 1.
    pub min: f32,
    pub max: f32,
}

impl ShortcutMarker {
    /// The chance a racer of `skill` (0 to 1) takes the shortcut, 0 to 1: from the minimum for the weakest to the
    /// maximum for the best. A stored value above 1 is a percentage. **[unconfirmed]** rule, the values are
    /// **[verified]**.
    pub fn chance(&self, skill: f32) -> f32 {
        let unit = |v: f32| if v > 1.0 { v / 100.0 } else { v };
        let (min, max) = (unit(self.min).clamp(0.0, 1.0), unit(self.max).clamp(0.0, 1.0));
        min + (max - min) * skill.clamp(0.0, 1.0)
    }
}

/// Every non-template shortcut marker, in database order.
pub(crate) fn shortcuts(db: &Database) -> Vec<ShortcutMarker> {
    instances(db).filter(|c| inherits(*c, "shortcut")).map(marker).collect()
}

fn marker(c: CollectionRef<'_>) -> ShortcutMarker {
    let p = placement(c);
    ShortcutMarker {
        key: c.key(),
        position: p.position,
        heading_deg: p.heading_deg,
        min: c.get_f32("ShortcutMinChance").unwrap_or(0.0),
        max: c.get_f32("ShortcutMaxChance").unwrap_or(1.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn marker(min: f32, max: f32) -> ShortcutMarker {
        ShortcutMarker { key: 1, position: Vec3::ZERO, heading_deg: 0.0, min, max }
    }

    #[test]
    fn the_chance_runs_from_the_minimum_to_the_maximum_with_skill() {
        let m = marker(0.2, 0.3);
        assert!((m.chance(0.0) - 0.2).abs() < 1e-6);
        assert!((m.chance(1.0) - 0.3).abs() < 1e-6);
        assert!((m.chance(0.5) - 0.25).abs() < 1e-6);
        assert_eq!(marker(0.0, 0.0).chance(1.0), 0.0, "a shortcut racers never take");
    }

    #[test]
    fn values_above_one_are_percentages() {
        let m = marker(80.0, 90.0);
        assert!((m.chance(0.0) - 0.8).abs() < 1e-6);
        assert!((m.chance(1.0) - 0.9).abs() < 1e-6);
        assert_eq!(marker(1.0, 1.0).chance(0.3), 1.0, "1 is a certainty, not a percent");
    }
}
