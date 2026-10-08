use glam::Vec3;

use crate::ground::{Ground, SurfaceGrip};

/// How far above the wheel contact point the ground ray starts, so a wheel pushed slightly into the
/// surface still finds it.
pub const PROBE_LIFT: f32 = 0.5;

/// What the ground probe of one wheel found.
#[derive(Clone, Copy, Debug)]
pub struct WheelContact {
    /// Where the ray met the ground.
    pub point: Vec3,
    pub normal: Vec3,
    /// Metres the contact point is below the surface, measured along the normal (negative = above it).
    pub penetration: f32,
    pub surface: SurfaceGrip,
}

/// Looks for the ground within `tolerance` metres below `point`.
pub fn probe(ground: &dyn Ground, point: Vec3, tolerance: f32) -> Option<WheelContact> {
    let hit = ground.hit(point + Vec3::Y * PROBE_LIFT, Vec3::NEG_Y, PROBE_LIFT + tolerance.max(0.0))?;
    if !hit.distance.is_finite() || hit.normal.y <= 0.0 {
        return None;
    }
    Some(WheelContact {
        point: point + Vec3::Y * (PROBE_LIFT - hit.distance),
        normal: hit.normal,
        penetration: (PROBE_LIFT - hit.distance) * hit.normal.y,
        surface: hit.surface,
    })
}

/// A wheel's compression for this step.
#[derive(Clone, Copy, Debug)]
pub struct Compression {
    /// Spring compression in metres, in `0..=travel`.
    pub value: f32,
    /// How far the contact is beyond the travel; the body is lifted by the largest of these.
    pub lift_need: f32,
    /// How flat the wheel sits on the ground relative to the body (0..1).
    pub upness: f32,
    /// Pressed onto flat enough ground: its spring and tire act.
    pub loaded: bool,
}

/// Compression from the ride height and the penetration: `ride * upness + penetration`, clamped to the
/// travel.
pub fn compression(ride: f32, travel: f32, upness: f32, penetration: f32) -> Compression {
    let raw = ride * upness + penetration;
    let lift_need = (raw - travel).max(0.0);
    let value = raw.clamp(0.0, travel);
    Compression { value, lift_need, upness, loaded: value > 0.0 && upness > 0.2 }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ground::FlatGround;

    #[test]
    fn probe_measures_the_gap_to_the_ground() {
        let g = FlatGround::new(0.0);
        let above = probe(&g, Vec3::new(0.0, 0.1, 0.0), 0.3).unwrap();
        assert!((above.penetration + 0.1).abs() < 1e-6);
        let below = probe(&g, Vec3::new(0.0, -0.05, 0.0), 0.3).unwrap();
        assert!((below.penetration - 0.05).abs() < 1e-6);
        assert!(probe(&g, Vec3::new(0.0, 0.5, 0.0), 0.3).is_none(), "beyond the tolerance");
    }

    #[test]
    fn compression_is_ride_height_minus_the_gap() {
        let c = compression(0.1, 0.12, 1.0, -0.06);
        assert!((c.value - 0.04).abs() < 1e-6 && c.loaded && c.lift_need == 0.0);
    }

    #[test]
    fn compression_clamps_and_asks_for_a_lift() {
        let c = compression(0.1, 0.12, 1.0, 0.1);
        assert_eq!(c.value, 0.12);
        assert!((c.lift_need - 0.08).abs() < 1e-6);
    }

    #[test]
    fn a_wheel_off_the_ground_is_free() {
        let c = compression(0.1, 0.12, 1.0, -0.2);
        assert_eq!(c.value, 0.0);
        assert!(!c.loaded);
    }

    #[test]
    fn a_steep_wall_does_not_load_the_wheel() {
        let c = compression(0.1, 0.12, 0.1, 0.0);
        assert!(!c.loaded);
    }
}
