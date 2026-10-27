//! Authored scenery flares. Behavior: `docs/specs/environment-prop-lights.md`.

use blackbox_attrib::Database;
use blackbox_solid::Solid;
use glam::Vec3;

use crate::car::lights::FlareProfile;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Orange,
    Red,
    Blue,
    AmberBlink,
    RedBlink,
    GreenBlink,
}

impl Kind {
    pub const ALL: [Self; 6] =
        [Self::Orange, Self::Red, Self::Blue, Self::AmberBlink, Self::RedBlink, Self::GreenBlink];

    pub fn from_parameter(parameter: i32) -> Option<Self> {
        usize::try_from(parameter).ok().and_then(|i| Self::ALL.get(i)).copied()
    }

    pub fn profile_name(self) -> &'static str {
        match self {
            Self::Orange => "catseye_orange",
            Self::Red => "catseye_red",
            Self::Blue => "catseye_blue",
            Self::AmberBlink => "traffic_light",
            Self::RedBlink => "traffic_light_red",
            Self::GreenBlink => "green_flare",
        }
    }

    pub fn directional(self) -> bool {
        matches!(self, Self::AmberBlink | Self::RedBlink | Self::GreenBlink)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Marker {
    /// Marker translation with the solid pivot translation removed, in model space.
    pub position: Vec3,
    pub kind: Kind,
}

/// Unsupported roles and invalid positions cannot produce environment lights.
pub fn markers(solid: &Solid) -> Vec<Marker> {
    let pivot = Vec3::from_slice(&solid.transform[12..15]);
    let min = Vec3::from_array(solid.bounds_min);
    let max = Vec3::from_array(solid.bounds_max);
    let bounds_valid = min.is_finite() && max.is_finite() && min.cmple(max).all() && min != max;
    solid
        .markers
        .iter()
        .filter_map(|marker| {
            let kind = Kind::from_parameter(marker.int_param)?;
            let position = Vec3::from_array(marker.translation()) - pivot;
            if !position.is_finite() {
                return None;
            }
            if bounds_valid && (min - position).max(position - max).max(Vec3::ZERO).length_squared() > 100.0 {
                return None;
            }
            Some(Marker { position, kind })
        })
        .collect()
}

/// No appearance defaults: missing profiles remain absent.
pub fn profiles(db: &Database) -> Vec<(Kind, FlareProfile)> {
    Kind::ALL
        .into_iter()
        .filter_map(|kind| {
            let profile = FlareProfile::read(db.collection("light_flares_cg", kind.profile_name())?)?;
            Some((kind, profile))
        })
        .collect()
}
