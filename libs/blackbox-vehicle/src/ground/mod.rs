//! What the vehicle drives on: a ray-cast trait the caller implements over its own world, the surface
//! grip multipliers a hit carries, and two trivial grounds for tests and examples.

use glam::Vec3;

/// Grip multipliers of a road surface (the attribute class `simsurface`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SurfaceGrip {
    /// Multiplies the lateral tire force.
    pub lateral: f32,
    /// Multiplies the longitudinal (drive and brake) tire force.
    pub drive: f32,
    /// Multiplies the rolling friction of a spinning wheel.
    pub rolling: f32,
}

impl SurfaceGrip {
    /// Dry tarmac: all multipliers 1.
    pub const DEFAULT: Self = Self { lateral: 1.0, drive: 1.0, rolling: 1.0 };
}

impl Default for SurfaceGrip {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// A ray hit on the ground.
#[derive(Clone, Copy, Debug)]
pub struct GroundHit {
    /// Distance from the ray origin along the (unit) direction.
    pub distance: f32,
    /// Unit surface normal, pointing out of the ground (up for a flat road).
    pub normal: Vec3,
    pub surface: SurfaceGrip,
}

/// The world as the vehicle sees it. Implement this over your collision data; `blackbox-collision` has a
/// ray cast that fits.
pub trait Ground {
    /// Casts a ray from `origin` along the unit vector `dir` for at most `max_distance` metres and returns
    /// the nearest ground surface it hits, if any. The vehicle casts straight down from slightly above each
    /// wheel contact point.
    fn hit(&self, origin: Vec3, dir: Vec3, max_distance: f32) -> Option<GroundHit>;
}

/// An infinite horizontal plane.
#[derive(Clone, Copy, Debug)]
pub struct FlatGround {
    pub height: f32,
    pub surface: SurfaceGrip,
}

impl FlatGround {
    pub fn new(height: f32) -> Self {
        Self { height, surface: SurfaceGrip::DEFAULT }
    }
}

impl Ground for FlatGround {
    fn hit(&self, origin: Vec3, dir: Vec3, max_distance: f32) -> Option<GroundHit> {
        if dir.y >= -1e-6 {
            return None;
        }
        let distance = (self.height - origin.y) / dir.y;
        (distance >= 0.0 && distance <= max_distance).then_some(GroundHit {
            distance,
            normal: Vec3::Y,
            surface: self.surface,
        })
    }
}

/// A plane tilted about the x axis: `height(z) = height + z * slope` (slope = rise over run).
#[derive(Clone, Copy, Debug)]
pub struct SlopedGround {
    pub height: f32,
    pub slope: f32,
    pub surface: SurfaceGrip,
}

impl Ground for SlopedGround {
    fn hit(&self, origin: Vec3, dir: Vec3, max_distance: f32) -> Option<GroundHit> {
        let normal = Vec3::new(0.0, 1.0, -self.slope).normalize();
        // Plane: normal . (p - (0, height, 0)) = 0.
        let denom = normal.dot(dir);
        if denom >= -1e-6 {
            return None;
        }
        let distance = normal.dot(Vec3::new(0.0, self.height, 0.0) - origin) / denom;
        (distance >= 0.0 && distance <= max_distance).then_some(GroundHit { distance, normal, surface: self.surface })
    }
}

/// No ground at all: the vehicle free-falls.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoGround;

impl Ground for NoGround {
    fn hit(&self, _origin: Vec3, _dir: Vec3, _max_distance: f32) -> Option<GroundHit> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flat_ground_hits_downward_rays_only() {
        let g = FlatGround::new(1.0);
        let h = g.hit(Vec3::new(5.0, 3.0, 0.0), Vec3::NEG_Y, 10.0).unwrap();
        assert!((h.distance - 2.0).abs() < 1e-6);
        assert_eq!(h.normal, Vec3::Y);
        assert!(g.hit(Vec3::new(0.0, 3.0, 0.0), Vec3::NEG_Y, 1.0).is_none());
        assert!(g.hit(Vec3::new(0.0, 3.0, 0.0), Vec3::Y, 10.0).is_none());
        assert!(g.hit(Vec3::new(0.0, 0.0, 0.0), Vec3::NEG_Y, 10.0).is_none(), "origin below the plane");
    }

    #[test]
    fn slope_rises_with_z() {
        let g = SlopedGround { height: 0.0, slope: 0.1, surface: SurfaceGrip::DEFAULT };
        let h = g.hit(Vec3::new(0.0, 5.0, 10.0), Vec3::NEG_Y, 10.0).unwrap();
        assert!((h.distance - 4.0).abs() < 1e-4, "{}", h.distance);
        assert!(h.normal.z < 0.0 && h.normal.y > 0.9);
    }
}
