//! The `[wheels]` table of a car manifest: where the car's model has its tyres.
//!
//! `[stats]` says what the tyres do (the physics); this table says where the model's wheels are, so the game draws the
//! wheels on the hubs of the model and not on hubs it works out from other numbers. A car that leaves the table out has
//! its wheels drawn from `[stats]` (the tyres' centres at plus and minus half of the wheelbase, as high as their radius).
//! Both axles are needed, in the car's own space (x forward, y left, z up, origin where the model's origin is).

use serde::Deserialize;

use super::ContentError;

/// One axle's left tyre as the model has it. The right tyre is its mirror.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tyre {
    /// How far forward of the car's origin the tyre's centre is (negative behind it).
    pub x_m: f32,
    /// Height of the tyre's centre above the ground the model stands on.
    pub z_m: f32,
    /// Distance from the car's centre line to the tyre's outer face.
    pub face_m: f32,
    pub radius_m: f32,
    pub width_m: f32,
}

/// The wheels of a car.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WheelLayout {
    pub front: Tyre,
    pub rear: Tyre,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RawTyre {
    x_m: f32,
    z_m: f32,
    face_m: f32,
    radius_m: f32,
    width_m: f32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Raw {
    front: RawTyre,
    rear: RawTyre,
}

fn ranged(value: f32, field: &'static str, low: f32, high: f32) -> Result<f32, ContentError> {
    if !(low..=high).contains(&value) {
        return Err(ContentError::Field { field, reason: "must be a number inside the range the docs give" });
    }
    Ok(value)
}

fn tyre(raw: RawTyre, fields: [&'static str; 5]) -> Result<Tyre, ContentError> {
    let [x, z, face, radius, width] = fields;
    Ok(Tyre {
        x_m: ranged(raw.x_m, x, -6.0, 6.0)?,
        z_m: ranged(raw.z_m, z, 0.05, 1.0)?,
        face_m: ranged(raw.face_m, face, 0.3, 2.0)?,
        radius_m: ranged(raw.radius_m, radius, 0.15, 0.8)?,
        width_m: ranged(raw.width_m, width, 0.1, 0.8)?,
    })
}

/// Check the table. A car with no table has none: its wheels come from `[stats]`.
pub(super) fn check(raw: Option<Raw>) -> Result<Option<WheelLayout>, ContentError> {
    let Some(raw) = raw else { return Ok(None) };
    let front = tyre(
        raw.front,
        [
            "wheels.front.x_m",
            "wheels.front.z_m",
            "wheels.front.face_m",
            "wheels.front.radius_m",
            "wheels.front.width_m",
        ],
    )?;
    let rear = tyre(
        raw.rear,
        ["wheels.rear.x_m", "wheels.rear.z_m", "wheels.rear.face_m", "wheels.rear.radius_m", "wheels.rear.width_m"],
    )?;
    if front.x_m <= rear.x_m {
        return Err(ContentError::Field { field: "wheels.front.x_m", reason: "the front axle is ahead of the rear" });
    }
    Ok(Some(WheelLayout { front, rear }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layout(text: &str) -> Result<Option<WheelLayout>, ContentError> {
        let raw: Option<Raw> = toml::from_str::<std::collections::HashMap<String, Raw>>(text)
            .map(|mut tables| tables.remove("wheels"))
            .map_err(|error| ContentError::Parse(error.to_string()))?;
        check(raw)
    }

    const GOOD: &str = "[wheels.front]
x_m = 1.3
z_m = 0.334
face_m = 0.932
radius_m = 0.32
width_m = 0.264
[wheels.rear]
x_m = -1.505
z_m = 0.356
face_m = 0.983
radius_m = 0.358
width_m = 0.346
";

    #[test]
    fn no_table_is_no_layout() {
        assert_eq!(layout("").unwrap(), None);
    }

    #[test]
    fn both_axles_are_read() {
        let layout = layout(GOOD).unwrap().unwrap();
        assert_eq!((layout.front.x_m, layout.front.face_m, layout.rear.width_m), (1.3, 0.932, 0.346));
    }

    #[test]
    fn a_number_out_of_range_names_its_key() {
        let error = layout(&GOOD.replace("0.358", "3.0")).unwrap_err();
        assert!(matches!(error, ContentError::Field { field: "wheels.rear.radius_m", .. }), "{error}");
    }

    #[test]
    fn an_axle_behind_the_other_is_refused() {
        let error = layout(&GOOD.replace("x_m = 1.3", "x_m = -2.0")).unwrap_err();
        assert!(matches!(error, ContentError::Field { field: "wheels.front.x_m", .. }), "{error}");
    }

    #[test]
    fn a_missing_axle_or_key_is_refused() {
        assert!(layout("[wheels.front]\nx_m = 1.0\n").is_err());
    }
}
