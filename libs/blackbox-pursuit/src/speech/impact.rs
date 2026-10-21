//! What a collision means to the cops' radio: how hard it was, whether it was a crash, and how the player's car met a
//! cop (`SoundAI::OnCollision`). Pure functions of the contact, so a game can feed them whatever its physics reports.
//! Spec: `docs/specs/ai-pursuit-speech.md` section 7.

use glam::Vec3;

use super::input::Impact;

/// Metres per second in one mile per hour.
pub const MPH: f32 = 0.447_04;

/// A hit is ignored below this value of the *squared* closing speed, which the original compares with 25 mph taken as
/// metres per second (11.176): in effect every hit closing slower than 3.34 m/s (7.5 mph) is dropped, not those under
/// 25 mph. **[decomp]**, the comparison is on `LengthSquare`; the spec text of "above 25 mph" was a misreading.
pub const CLOSING_GATE: f32 = 25.0 * MPH;

/// Whether a collision closes fast enough for the radio to take notice of it.
pub fn closing_enough(closing: Vec3) -> bool {
    closing.length_squared() >= CLOSING_GATE
}

/// How hard a collision was, 0 to 1: the speed the two bodies lost between them (`impulseA + impulseB`, metres per
/// second) between the two ends of the tuning's smash range (miles per hour).
pub fn intensity(delta_v: f32, range_mph: [f32; 2]) -> f32 {
    let (low, high) = (range_mph[0] * MPH, range_mph[1] * MPH);
    let span = high - low;
    if span <= 0.0 {
        return 0.0;
    }
    ((delta_v - low) / span).clamp(0.0, 1.0)
}

/// Whether a collision cost the player's car enough speed to count as a crash: the speed after it over the speed before
/// is under one minus the tuning's slowdown share. A car with no speed before it does not crash.
pub fn crashed(speed_before: f32, speed_after: f32, slowdown: f32) -> bool {
    if speed_before <= 0.0 {
        return false;
    }
    speed_after / speed_before < 1.0 - slowdown
}

/// The two cars of a player and cop contact, as the classification reads them. The arms are the contact point from each
/// car's centre in the car's own axes (x right, z forward), the dimensions each car's half extents in the same axes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Contact {
    /// The dot product of the player's forward vector and the cop's.
    pub forward_dot: f32,
    pub arm_cop: Vec3,
    pub arm_player: Vec3,
    pub dim_cop: Vec3,
    pub dim_player: Vec3,
    /// How fast each car moved along the contact normal before the hit (absolute values).
    pub normal_speed_cop: f32,
    pub normal_speed_player: f32,
    /// Each car's speed before the hit.
    pub speed_cop: f32,
    pub speed_player: f32,
}

/// How the player's car met the cop, and whether it was the player who rammed. **[decomp]**: cars that run the same way
/// (dot 0.7 or more) either side-swipe (both arms past three quarters of the half width; the player rams when it moved
/// faster into the contact) or one rear-ends the other (the player rams when it is the faster); cars across each other
/// (dot between -0.7 and 0.7) are a T-bone with the player's nose in; cars head to head (dot -0.7 or less) are head-on and
/// the player rams when it moved faster into the contact. A hit the cop caused is [`Impact::None`]. A player's nose in the
/// cop's side with the cop's nose in the player's is a ram the original has no wording for ([`Impact::Other`]).
pub fn classify(c: &Contact) -> Impact {
    if c.forward_dot >= 0.7 {
        return same_way(c);
    }
    if c.forward_dot > -0.7 {
        return across(c);
    }
    match c.normal_speed_player > c.normal_speed_cop {
        true => Impact::HeadOn,
        false => Impact::None,
    }
}

fn same_way(c: &Contact) -> Impact {
    let abeam = c.arm_cop.x.abs() > c.dim_cop.x * 0.75 && c.arm_player.x.abs() > c.dim_player.x * 0.75;
    if abeam {
        return match c.normal_speed_cop <= c.normal_speed_player {
            true => Impact::SideSwipe,
            false => Impact::None,
        };
    }
    let player_behind = c.arm_player.z >= c.dim_player.z * 0.75 && c.arm_cop.z <= c.dim_cop.z * -0.75;
    if player_behind && c.speed_player > c.speed_cop {
        return Impact::RearEnd;
    }
    Impact::None
}

fn across(c: &Contact) -> Impact {
    let nose_in = c.arm_player.z >= c.dim_player.z * 0.7;
    if !nose_in {
        return Impact::None;
    }
    match c.arm_cop.z >= c.dim_cop.z * 0.7 {
        true => Impact::Other,
        false => Impact::TBone,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DIM: Vec3 = Vec3::new(1.0, 0.7, 2.2);

    fn contact(forward_dot: f32) -> Contact {
        Contact {
            forward_dot,
            arm_cop: Vec3::ZERO,
            arm_player: Vec3::ZERO,
            dim_cop: DIM,
            dim_player: DIM,
            normal_speed_cop: 5.0,
            normal_speed_player: 10.0,
            speed_cop: 20.0,
            speed_player: 30.0,
        }
    }

    #[test]
    fn the_gate_is_on_the_squared_closing_speed() {
        assert!(!closing_enough(Vec3::new(3.3, 0.0, 0.0)), "3.3 m/s squared is under 11.176");
        assert!(closing_enough(Vec3::new(3.4, 0.0, 0.0)));
        assert!(closing_enough(Vec3::new(2.0, 0.0, 3.0)));
    }

    #[test]
    fn intensity_runs_from_ten_to_thirty_mph_of_speed_lost() {
        let range = [10.0, 30.0];
        assert_eq!(intensity(10.0 * MPH, range), 0.0);
        assert!((intensity(20.0 * MPH, range) - 0.5).abs() < 1e-5);
        assert_eq!(intensity(40.0 * MPH, range), 1.0);
        assert_eq!(intensity(1.0, [30.0, 10.0]), 0.0, "a broken range says nothing");
    }

    #[test]
    fn a_crash_loses_more_than_the_slowdown_share() {
        assert!(crashed(30.0, 9.0, 0.66), "30 to 9 is 30 percent left");
        assert!(!crashed(30.0, 11.0, 0.66), "37 percent left is a scrape");
        assert!(!crashed(0.0, 0.0, 0.66), "a car at rest does not crash");
    }

    #[test]
    fn running_the_same_way_and_abeam_is_a_side_swipe_when_the_player_closes_faster() {
        let mut c = contact(0.95);
        c.arm_cop.x = 0.9;
        c.arm_player.x = -0.9;
        assert_eq!(classify(&c), Impact::SideSwipe);
        c.normal_speed_cop = 12.0;
        assert_eq!(classify(&c), Impact::None, "the cop drove into the player");
    }

    #[test]
    fn the_faster_player_behind_the_cop_rear_ends_it() {
        let mut c = contact(0.9);
        c.arm_player.z = 2.0;
        c.arm_cop.z = -2.0;
        assert_eq!(classify(&c), Impact::RearEnd);
        c.speed_player = 10.0;
        assert_eq!(classify(&c), Impact::None, "a slower player is the one being run into");
    }

    #[test]
    fn the_players_nose_in_the_cops_side_is_a_t_bone() {
        let mut c = contact(0.0);
        c.arm_player.z = 2.0;
        assert_eq!(classify(&c), Impact::TBone);
        c.arm_cop.z = 2.0;
        assert_eq!(classify(&c), Impact::Other, "both noses in");
        c.arm_player.z = 0.0;
        assert_eq!(classify(&c), Impact::None, "the cop's nose in the player's side is the cop's ram");
    }

    #[test]
    fn head_to_head_rams_when_the_player_moves_faster_into_it() {
        let mut c = contact(-0.9);
        assert_eq!(classify(&c), Impact::HeadOn);
        c.normal_speed_player = 3.0;
        assert_eq!(classify(&c), Impact::None);
    }
}
