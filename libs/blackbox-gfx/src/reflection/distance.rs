//! How far the reflection captures reach. The original sets a far limit for the car cube (100 m) and for the rear-view
//! mirror (192 m). `unlimited` keeps the port's reach, which is limited only by the scenery that is streamed in.
//!
//! A limit is applied to each object's origin, not to its triangles: an object whose origin lies beyond the limit is
//! dropped whole, and one whose origin is inside it is drawn whole, even where it crosses the limit.

use std::borrow::Cow;

use glam::Vec3;

use crate::Instance;

/// The car cube's far limit in metres, as the original sets it for its six cameras.
pub const CAR_FAR: f32 = 100.0;
/// The rear-view mirror's far limit in metres, as the original sets it.
pub const MIRROR_FAR: f32 = 192.0;

/// The reach of the reflection captures.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum ReflectionDistance {
    /// No limit: the port's current reach.
    #[default]
    Unlimited,
    /// The original limits: [`CAR_FAR`] for the car cube and [`MIRROR_FAR`] for the mirror.
    Original,
    /// A user-selected limit, in metres, for both captures.
    Meters(u16),
}

impl ReflectionDistance {
    pub const ALL: [Self; 7] = [
        Self::Unlimited,
        Self::Original,
        Self::Meters(150),
        Self::Meters(200),
        Self::Meters(300),
        Self::Meters(500),
        Self::Meters(1000),
    ];

    pub fn name(self) -> Cow<'static, str> {
        match self {
            Self::Unlimited => "unlimited".into(),
            Self::Original => "original".into(),
            Self::Meters(metres) => format!("{metres}m").into(),
        }
    }

    /// The car cube's far limit in metres, or `None` for no limit.
    pub const fn car_far(self) -> Option<f32> {
        match self {
            Self::Unlimited => None,
            Self::Original => Some(CAR_FAR),
            Self::Meters(metres) => Some(metres as f32),
        }
    }

    /// The rear-view mirror's far limit in metres, or `None` for no limit.
    pub const fn mirror_far(self) -> Option<f32> {
        match self {
            Self::Unlimited => None,
            Self::Original => Some(MIRROR_FAR),
            Self::Meters(metres) => Some(metres as f32),
        }
    }
}

impl std::str::FromStr for ReflectionDistance {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let text = s.trim().to_ascii_lowercase();
        if let Some(metres) = text.strip_suffix('m').and_then(|v| v.trim().parse::<u16>().ok())
            && (25..=2000).contains(&metres)
        {
            return Ok(Self::Meters(metres));
        }
        Self::ALL
            .into_iter()
            .find(|d| d.name() == text)
            .ok_or_else(|| format!("expected unlimited, original or 25m–2000m, got {s:?}"))
    }
}

impl std::fmt::Display for ReflectionDistance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.name())
    }
}

/// Whether an object whose origin is `centre` lies within `far` metres of `origin`. No limit keeps every object.
pub fn within(origin: Vec3, centre: Vec3, far: Option<f32>) -> bool {
    match far {
        None => true,
        Some(far) => origin.distance_squared(centre) <= far * far,
    }
}

/// The instances whose origin lies within `far` metres of `origin`. With no limit the list is borrowed as it is.
pub fn cull_instances(instances: &[Instance], origin: Vec3, far: Option<f32>) -> Cow<'_, [Instance]> {
    if far.is_none() {
        return Cow::Borrowed(instances);
    }
    Cow::Owned(instances.iter().copied().filter(|i| within(origin, i.transform.w_axis.truncate(), far)).collect())
}

#[cfg(test)]
mod tests {
    use glam::Mat4;

    use super::*;
    use crate::MeshHandle;

    fn at(x: f32) -> Instance {
        Instance::new(MeshHandle::from_raw(1), Mat4::from_translation(Vec3::new(x, 0.0, 0.0)))
    }

    #[test]
    fn the_default_is_the_port_reach_and_the_original_has_the_stock_limits() {
        assert_eq!(ReflectionDistance::default(), ReflectionDistance::Unlimited);
        assert_eq!(ReflectionDistance::Unlimited.car_far(), None);
        assert_eq!(ReflectionDistance::Unlimited.mirror_far(), None);
        assert_eq!(ReflectionDistance::Original.car_far(), Some(100.0));
        assert_eq!(ReflectionDistance::Original.mirror_far(), Some(192.0));
    }

    #[test]
    fn distance_text_round_trips_and_rejects_other_names() {
        for distance in ReflectionDistance::ALL {
            assert_eq!(distance.to_string().parse::<ReflectionDistance>().unwrap(), distance);
        }
        assert_eq!(" ORIGINAL ".parse::<ReflectionDistance>().unwrap(), ReflectionDistance::Original);
        for text in ["", "100", "far", "384"] {
            assert!(text.parse::<ReflectionDistance>().is_err(), "{text:?}");
        }
    }

    #[test]
    fn custom_distance_round_trips_and_reaches_both_capture_paths() {
        let d: ReflectionDistance = " 275 m ".parse().unwrap();
        assert_eq!(d.to_string(), "275m");
        assert_eq!((d.car_far(), d.mirror_far()), (Some(275.0), Some(275.0)));
        for text in ["0m", "24m", "2001m", "NaNm", "-100m"] {
            assert!(text.parse::<ReflectionDistance>().is_err());
        }
    }

    #[test]
    fn a_limit_keeps_origins_inside_it_and_no_limit_keeps_all() {
        let origin = Vec3::ZERO;
        assert!(within(origin, Vec3::new(1000.0, 0.0, 0.0), None));
        assert!(within(origin, Vec3::new(99.0, 0.0, 0.0), Some(100.0)));
        assert!(within(origin, Vec3::new(100.0, 0.0, 0.0), Some(100.0)), "the limit itself is inside");
        assert!(!within(origin, Vec3::new(0.0, 101.0, 0.0), Some(100.0)));
    }

    #[test]
    fn no_limit_borrows_the_instances_and_a_limit_copies_the_near_ones() {
        let list = [at(10.0), at(150.0), at(50.0)];
        let all = cull_instances(&list, Vec3::ZERO, None);
        assert!(matches!(all, Cow::Borrowed(_)));
        assert_eq!(all.len(), 3);
        let near = cull_instances(&list, Vec3::ZERO, Some(100.0));
        assert!(matches!(near, Cow::Owned(_)));
        let kept: Vec<f32> = near.iter().map(|i| i.transform.w_axis.x).collect();
        assert_eq!(kept, [10.0, 50.0]);
    }
}
