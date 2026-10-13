//! Box triggers: the collections that inherit `boxtrigger` (finish lines, checkpoints and the other box-shaped triggers).
//! Spec: `docs/formats/gameplay-races.md` section 3.1 and open question 1.

use blackbox_attrib::{CollectionRef, Database};
use glam::Vec3;

use super::{inherits, instances, name_of, placement, text};

/// A box-shaped trigger.
#[derive(Debug, Clone, PartialEq)]
pub struct BoxTrigger {
    /// The collection's key (the hash of its name); races refer to triggers by this key.
    pub key: u32,
    /// For example `race_bin_10/10_2_1_sprint/checkpoint1`.
    pub name: String,
    /// `Position` in physics space (the box centre, [`super::to_world`]).
    pub position: Vec3,
    /// `Rotation` in degrees; the heading is [`super::world_heading`].
    pub heading_deg: f32,
    /// `Dimensions` as stored: three extents. Checkpoints are `(2, W, 40)` or `(40, 40, 40)`; finish lines
    /// `(2 or 3, 25..65, 3..40)`. Which extent lies along the heading is unconfirmed (see the format doc).
    pub dimensions: [f32; 3],
    /// `OneShot`: the trigger fires once per race.
    pub one_shot: bool,
    /// `FireOnExit`: the trigger fires when the car leaves the box, not when it enters.
    pub fire_on_exit: bool,
    /// `ParticleEffect`: an effect shown at the trigger, for example `fxgame_flare_red` on finish lines.
    pub particle_effect: Option<String>,
    /// `FlareSpacing`: the distance between the flares of `particle_effect`, when set.
    pub flare_spacing: Option<f32>,
}

/// Every non-template box trigger, in database order.
pub(crate) fn triggers(db: &Database) -> Vec<BoxTrigger> {
    instances(db).filter(|c| inherits(*c, "boxtrigger")).map(trigger).collect()
}

fn trigger(c: CollectionRef<'_>) -> BoxTrigger {
    let p = placement(c);
    BoxTrigger {
        key: c.key(),
        name: name_of(c),
        position: p.position,
        heading_deg: p.heading_deg,
        dimensions: c.get_vector3("Dimensions").unwrap_or_default(),
        one_shot: c.get_bool("OneShot").unwrap_or(false),
        fire_on_exit: c.get_bool("FireOnExit").unwrap_or(false),
        particle_effect: text(c, "ParticleEffect"),
        flare_spacing: c.get_f32("FlareSpacing"),
    }
}
