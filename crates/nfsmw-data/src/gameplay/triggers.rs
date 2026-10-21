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
    /// `FlareSpacing`: how far each of the two flares of `particle_effect` stands from the trigger's centre, along its
    /// width axis (so the flares are twice this apart), when set. `docs/specs/race-finish-effects.md` section 1.
    pub flare_spacing: Option<f32>,
    /// `TimeBonus` (an `Int32`): seconds the race clock gains when the player reaches the checkpoint. Set on the
    /// `time_bonus_checkpoint` triggers of the tollbooth races (25, 35 and 160 s in `2.4.3`). 0 when unset.
    pub time_bonus: f32,
    /// `ResetsPlayer`: crossing it sends the player back to the grid (the wrong-way trigger).
    pub resets_player: bool,
}

impl BoxTrigger {
    /// Whether `point` (physics space) is inside the box. The box is centred on `position` and turned by
    /// `heading_deg` about the up axis; `dimensions` are full extents: the first along the heading, the second across
    /// it (horizontally), the third up. Spec: `docs/specs/world-markers.md` section 3. **[unconfirmed]** The axis
    /// assignment is fitted to the data (a finish line is a thin gate across the road: its first extent is 2 to 3 m),
    /// not read from the trigger code.
    pub fn contains(&self, point: Vec3) -> bool {
        let along = super::world_heading(self.heading_deg);
        let across = Vec3::Y.cross(along);
        let offset = point - self.position;
        let [length, width, height] = self.dimensions.map(|extent| extent * 0.5);
        offset.dot(along).abs() <= length && offset.dot(across).abs() <= width && offset.y.abs() <= height
    }
}

/// A sphere trigger (class `radiustrigger`): the speed traps are of this kind.
#[derive(Debug, Clone, PartialEq)]
pub struct SphereTrigger {
    /// The collection's key (the hash of its name); a race's `SpeedTrapList` refers to it.
    pub key: u32,
    /// For example `race_bin_01/1_5_3_speedtrap/speedtrap1`.
    pub name: String,
    /// `Position` in physics space.
    pub position: Vec3,
    /// `Radius`, metres.
    pub radius: f32,
    /// `ThreshholdSpeed` (sic): the speed in km/h a speed trap needs to pay (88 in `1.5.3`). 0 when unset.
    pub threshold_speed: f32,
}

/// Every non-template sphere trigger, in database order.
pub(crate) fn spheres(db: &Database) -> Vec<SphereTrigger> {
    instances(db)
        .filter(|c| inherits(*c, "radiustrigger"))
        .map(|c| SphereTrigger {
            key: c.key(),
            name: name_of(c),
            position: placement(c).position,
            radius: c.get_f32("Radius").unwrap_or(0.0),
            threshold_speed: c.get_f32("ThreshholdSpeed").unwrap_or(0.0),
        })
        .collect()
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
        time_bonus: c.get_i32("TimeBonus").map_or(0.0, |seconds| seconds as f32),
        resets_player: c.get_bool("ResetsPlayer").unwrap_or(false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gate(heading_deg: f32, dimensions: [f32; 3]) -> BoxTrigger {
        BoxTrigger {
            key: 1,
            name: "gate".to_string(),
            position: Vec3::new(10.0, 0.0, -5.0),
            heading_deg,
            dimensions,
            one_shot: false,
            fire_on_exit: false,
            particle_effect: None,
            flare_spacing: None,
            time_bonus: 0.0,
            resets_player: false,
        }
    }

    #[test]
    fn the_first_extent_lies_along_the_heading_at_zero_degrees() {
        // Heading 0 is +z; the thin 2 m extent runs along it, the 40 m width across (+x), the 40 m height up.
        let b = gate(0.0, [2.0, 40.0, 40.0]);
        assert!(b.contains(Vec3::new(10.0, 0.0, -4.1)), "inside, 0.9 m ahead of the centre");
        assert!(!b.contains(Vec3::new(10.0, 0.0, -3.9)), "1.1 m ahead is outside the 2 m thickness");
        assert!(b.contains(Vec3::new(29.9, 0.0, -5.0)), "19.9 m across is inside the 40 m width");
        assert!(!b.contains(Vec3::new(30.1, 0.0, -5.0)));
        assert!(b.contains(Vec3::new(10.0, 19.9, -5.0)), "19.9 m up is inside the 40 m height");
        assert!(!b.contains(Vec3::new(10.0, 20.1, -5.0)));
    }

    #[test]
    fn the_box_turns_with_the_heading() {
        // Heading 90 is -x: the thin extent now runs along x, the width along z.
        let b = gate(90.0, [2.0, 40.0, 40.0]);
        assert!(b.contains(Vec3::new(9.1, 0.0, -5.0)));
        assert!(!b.contains(Vec3::new(11.1, 0.0, -5.0)));
        assert!(b.contains(Vec3::new(10.0, 0.0, 14.9)), "a 40 m width along z");
        assert!(!b.contains(Vec3::new(10.0, 0.0, 15.1)));
    }

    #[test]
    fn a_finish_line_is_thin_along_its_heading_and_wide_across() {
        // The 10_2_1_sprint finish line's stored extents (3, 65, 5): 3 m thick, 65 m wide, 5 m high.
        let b = gate(0.0, [3.0, 65.0, 5.0]);
        assert!(b.contains(Vec3::new(10.0 + 32.0, 2.0, -5.0)), "32 m across: inside the road");
        assert!(!b.contains(Vec3::new(10.0, 2.0, -5.0 + 2.0)), "2 m along: past the 1.5 m half thickness");
    }
}
