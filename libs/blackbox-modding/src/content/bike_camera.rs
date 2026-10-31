//! The `[bike.camera]` table: where the game's cameras sit on a bike (`docs/specs/ride6-bike-cameras.md`,
//! `docs/modding/bikes.md`). The game's own camera modes are made for a car (a bonnet, a bumper, a car-sized chase); a bike
//! carries its own numbers for them. Space is the model's: x forward, y left, z up, metres, the origin on the ground. Every
//! table and key is optional: what the manifest leaves out is derived from the bike's wheelbase (`bike_camera` of
//! `nfsmw_data::car::mods`), so a bike with no table still has a camera that fits it.
//!
//! - `chase_near` and `chase_far` are the third-person views behind the bike. They give the game's `close` and `far` views
//!   (and, scaled, the `superfar`, `drift` and `pursuit` ones) and the size of its custom chase and dynamic-FOV cameras.
//! - `front` is the camera ahead of the bike, outside the rider's helmet, that looks back at the bike and the road.
//! - `helmet` is the first-person camera inside the rider's helmet, at his eyes, with his helmet and visor not drawn.
//!
//! The checks of the ranges catch typing mistakes (a field of view of 400 degrees, a camera 100 m away), not unusual bikes.

use serde::Deserialize;

use super::ContentError;
use super::bike_check::{all_ranged, ranged};

/// The cameras of a bike (`[bike.camera]`).
#[derive(Debug, Clone, PartialEq, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Camera {
    pub chase_near: Option<Chase>,
    pub chase_far: Option<Chase>,
    pub front: Option<Front>,
    pub helmet: Option<Helmet>,
}

/// A third-person view behind the bike (`[bike.camera.chase_near]`, `[bike.camera.chase_far]`).
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Chase {
    /// How far behind the model's origin the camera is, m.
    pub distance_m: f32,
    /// How high above the ground, m.
    pub height_m: f32,
    /// The point it looks at: this far ahead of the origin and this high, m.
    pub target_ahead_m: f32,
    pub target_height_m: f32,
    /// The horizontal field of view, degrees.
    pub fov_deg: f32,
}

/// The camera ahead of the bike (`[bike.camera.front]`).
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Front {
    /// Where it is fixed to the bike, m (x forward, y left, z up).
    pub position_m: [f32; 3],
    /// Whether it looks back at the bike (true) or ahead down the road (false).
    #[serde(default)]
    pub look_back: bool,
    /// Its tilt from the bike's own axis, degrees, positive looking up.
    #[serde(default)]
    pub pitch_deg: f32,
    pub fov_deg: f32,
    /// The near plane, m.
    #[serde(default = "default_near")]
    pub near_m: f32,
    /// How much of the bike's pitch and roll the camera takes, 0 (it keeps the horizon) to 1 (it is bolted to the bike).
    #[serde(default)]
    pub pitch_factor: f32,
    #[serde(default)]
    pub roll_factor: f32,
}

/// The camera inside the helmet (`[bike.camera.helmet]`).
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Helmet {
    /// The rider's bones the eye point is the middle of, found by name in his skeleton; if none is found, `head_bone` and
    /// `head_eye_offset_m` place it.
    #[serde(default)]
    pub eye_bones: Vec<String>,
    #[serde(default)]
    pub head_bone: String,
    /// From the head bone to the eyes, in the model's axes, m.
    #[serde(default)]
    pub head_eye_offset_m: [f32; 3],
    /// The names of the rider's meshes that are not drawn while this camera is in use (the helmet and the visor).
    #[serde(default)]
    pub hide_meshes: Vec<String>,
    /// Where the eyes are on a bike with no rider, or while he is off the bike, m (x forward, y left, z up).
    pub fixed_position_m: [f32; 3],
    /// The tilt from the bike's own axis, degrees, positive looking up.
    #[serde(default)]
    pub pitch_deg: f32,
    pub fov_deg: f32,
    #[serde(default = "default_near")]
    pub near_m: f32,
    /// How much of the bike's pitch and roll the camera takes (see [`Front`]).
    #[serde(default)]
    pub pitch_factor: f32,
    #[serde(default)]
    pub roll_factor: f32,
    /// The roll of the bike (degrees) the camera follows beyond `roll_factor`, up to this much and no more; 0: none.
    #[serde(default)]
    pub follow_roll_max_deg: f32,
}

fn default_near() -> f32 {
    0.1
}

/// The key names of the checks of a chase view, in the order of its fields.
const NEAR_KEYS: [&str; 5] = [
    "bike.camera.chase_near.distance_m",
    "bike.camera.chase_near.height_m",
    "bike.camera.chase_near.target_ahead_m",
    "bike.camera.chase_near.target_height_m",
    "bike.camera.chase_near.fov_deg",
];
const FAR_KEYS: [&str; 5] = [
    "bike.camera.chase_far.distance_m",
    "bike.camera.chase_far.height_m",
    "bike.camera.chase_far.target_ahead_m",
    "bike.camera.chase_far.target_height_m",
    "bike.camera.chase_far.fov_deg",
];

impl Chase {
    fn check(&self, keys: &[&'static str; 5]) -> Result<(), ContentError> {
        ranged(self.distance_m, keys[0], 0.5, 30.0)?;
        ranged(self.height_m, keys[1], 0.1, 15.0)?;
        ranged(self.target_ahead_m, keys[2], -5.0, 10.0)?;
        ranged(self.target_height_m, keys[3], 0.0, 5.0)?;
        ranged(self.fov_deg, keys[4], 20.0, 150.0)?;
        Ok(())
    }
}

impl Camera {
    /// Refuses a number outside the range a typing mistake would leave it.
    pub(super) fn check(&self) -> Result<(), ContentError> {
        if let Some(chase) = &self.chase_near {
            chase.check(&NEAR_KEYS)?;
        }
        if let Some(chase) = &self.chase_far {
            chase.check(&FAR_KEYS)?;
        }
        if let Some(front) = &self.front {
            all_ranged(&front.position_m, "bike.camera.front.position_m", -10.0, 10.0)?;
            ranged(front.pitch_deg, "bike.camera.front.pitch_deg", -89.0, 89.0)?;
            ranged(front.fov_deg, "bike.camera.front.fov_deg", 20.0, 150.0)?;
            ranged(front.near_m, "bike.camera.front.near_m", 0.01, 2.0)?;
            ranged(front.pitch_factor, "bike.camera.front.pitch_factor", 0.0, 1.0)?;
            ranged(front.roll_factor, "bike.camera.front.roll_factor", 0.0, 1.0)?;
        }
        if let Some(helmet) = &self.helmet {
            all_ranged(&helmet.head_eye_offset_m, "bike.camera.helmet.head_eye_offset_m", -1.0, 1.0)?;
            all_ranged(&helmet.fixed_position_m, "bike.camera.helmet.fixed_position_m", -10.0, 10.0)?;
            ranged(helmet.pitch_deg, "bike.camera.helmet.pitch_deg", -89.0, 89.0)?;
            ranged(helmet.fov_deg, "bike.camera.helmet.fov_deg", 20.0, 150.0)?;
            ranged(helmet.near_m, "bike.camera.helmet.near_m", 0.01, 2.0)?;
            ranged(helmet.pitch_factor, "bike.camera.helmet.pitch_factor", 0.0, 1.0)?;
            ranged(helmet.roll_factor, "bike.camera.helmet.roll_factor", 0.0, 1.0)?;
            ranged(helmet.follow_roll_max_deg, "bike.camera.helmet.follow_roll_max_deg", 0.0, 90.0)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: &str = r#"
[chase_near]
distance_m = 3.3
height_m = 1.0
target_ahead_m = 0.3
target_height_m = 0.7
fov_deg = 85.5

[chase_far]
distance_m = 4.0
height_m = 1.45
target_ahead_m = 0.3
target_height_m = 0.7
fov_deg = 85.5

[front]
position_m = [1.9, 0.0, 1.1]
look_back = true
fov_deg = 104.5
pitch_factor = 0.3
roll_factor = 0.8

[helmet]
eye_bones = ["eye_l", "eye_r"]
hide_meshes = ["helmet"]
fixed_position_m = [0.1, 0.0, 1.2]
pitch_deg = -10.0
fov_deg = 91.5
pitch_factor = 0.25
follow_roll_max_deg = 5.0
"#;

    const FRONT: &str = "[front]\nposition_m = [1.0, 0.0, 1.0]\nfov_deg = 90.0\n";

    #[test]
    fn an_empty_table_is_valid_and_has_no_camera() {
        let table: Camera = toml::from_str("").unwrap();
        assert_eq!(table, Camera::default());
        table.check().unwrap();
    }

    #[test]
    fn every_key_is_read_and_the_unset_ones_take_their_defaults() {
        let table: Camera = toml::from_str(ALL).unwrap();
        table.check().unwrap();
        assert_eq!(table.chase_far.as_ref().map(|c| c.distance_m), Some(4.0));
        let front = table.front.unwrap();
        assert!(front.look_back);
        assert_eq!((front.near_m, front.pitch_deg), (0.1, 0.0), "the near plane and the tilt default");
        let helmet = table.helmet.unwrap();
        assert_eq!(helmet.eye_bones, ["eye_l", "eye_r"]);
        assert_eq!((helmet.roll_factor, helmet.follow_roll_max_deg), (0.0, 5.0));
    }

    #[test]
    fn an_unknown_key_and_a_number_out_of_range_are_refused_with_the_key_named() {
        assert!(toml::from_str::<Camera>(&format!("{FRONT}zoom = 2.0")).is_err());
        let table: Camera = toml::from_str(&FRONT.replace("90.0", "400.0")).unwrap();
        assert!(table.check().unwrap_err().to_string().contains("bike.camera.front.fov_deg"));
        let table: Camera = toml::from_str(&ALL.replace("distance_m = 3.3", "distance_m = 300.0")).unwrap();
        assert!(table.check().unwrap_err().to_string().contains("bike.camera.chase_near.distance_m"));
        let table: Camera = toml::from_str(&ALL.replace("pitch_factor = 0.25", "pitch_factor = 2.0")).unwrap();
        assert!(table.check().unwrap_err().to_string().contains("bike.camera.helmet.pitch_factor"));
    }
}
