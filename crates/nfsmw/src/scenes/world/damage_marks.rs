//! Visible damage over a car: scuffs where the paint is scraped, and crack lines on the glass.
//! The original's damage textures are not in the port yet, so these are procedural quads on the car's
//! surface. Pure geometry in the model frame (x forward, y left, z up), sized from the body's half
//! extents; the caller places them with the car's pose.

use blackbox_damage::{Damage, MAX_LEVEL};
use glam::Vec3;
use nfsmw_data::car::damage::ZONE_HAS_GLASS;

/// Where each zone sits on the car, as a direction from the centre (`DZ_*` order).
const ANCHORS: [Vec3; 10] = [
    Vec3::X,
    Vec3::NEG_X,
    Vec3::Y,
    Vec3::NEG_Y,
    Vec3::new(1.0, 1.0, 0.0),
    Vec3::new(1.0, -1.0, 0.0),
    Vec3::new(-1.0, 1.0, 0.0),
    Vec3::new(-1.0, -1.0, 0.0),
    Vec3::Z,
    Vec3::NEG_Z,
];
/// Scuff size per paint level, metres: the square side is this plus `SCUFF_STEP` per level.
const SCUFF_SIZE: f32 = 0.18;
const SCUFF_STEP: f32 = 0.08;
/// Crack length and half-width, metres; each glass level adds two cracks.
const CRACK_LENGTH: f32 = 0.42;
const CRACK_HALF_WIDTH: f32 = 0.008;
/// How far in front of the body's face the marks sit, metres.
const LIFT: f32 = 0.01;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mark {
    Scuff,
    Crack,
}

/// One mark: four corners in the model frame and what it is.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MarkQuad {
    pub corners: [Vec3; 4],
    pub mark: Mark,
}

/// Every mark for `damage` on a body whose model-frame half extents are `half`.
pub fn marks(damage: &Damage, half: Vec3) -> Vec<MarkQuad> {
    let mut out = Vec::new();
    for zone in 0..damage.zone_count().min(ANCHORS.len()) {
        let levels = damage.zone(zone);
        let face = Face::of(ANCHORS[zone], half);
        if levels.paint > 0 {
            let side = SCUFF_SIZE + SCUFF_STEP * f32::from(levels.paint);
            out.push(MarkQuad { corners: face.square(side * 0.5, zone), mark: Mark::Scuff });
        }
        if ZONE_HAS_GLASS[zone] {
            for crack in 0..usize::from(levels.glass.min(MAX_LEVEL)) * 2 {
                out.push(MarkQuad { corners: face.crack(crack, zone), mark: Mark::Crack });
            }
        }
    }
    out
}

/// A face of the body: the plane the marks lie in, and the centre of a zone on it.
struct Face {
    /// The face's centre, on the body's surface, offset by `LIFT`.
    centre: Vec3,
    /// The two in-plane axes.
    u: Vec3,
    v: Vec3,
}

impl Face {
    fn of(anchor: Vec3, half: Vec3) -> Self {
        let axis = (0..3).max_by(|&a, &b| anchor[a].abs().total_cmp(&anchor[b].abs())).unwrap_or(0);
        let sign = anchor[axis].signum();
        let mut centre = anchor * half * 0.6;
        centre[axis] = sign * (half[axis] + LIFT);
        let mut u = Vec3::ZERO;
        let mut v = Vec3::ZERO;
        u[(axis + 1) % 3] = 1.0;
        v[(axis + 2) % 3] = 1.0;
        Self { centre, u, v }
    }

    /// A square of half-side `r`, centred on the zone (a distinct corner per zone keeps them apart).
    fn square(&self, r: f32, zone: usize) -> [Vec3; 4] {
        let jitter = (zone as f32 * 0.37).sin() * 0.05;
        let c = self.centre + self.u * jitter;
        [
            c - self.u * r - self.v * r,
            c + self.u * r - self.v * r,
            c + self.u * r + self.v * r,
            c - self.u * r + self.v * r,
        ]
    }

    /// One crack: a thin line from the zone's centre in a fixed direction on the face.
    fn crack(&self, index: usize, zone: usize) -> [Vec3; 4] {
        let angle = (index as f32 + zone as f32 * 0.7) * 1.9;
        let dir = self.u * angle.cos() + self.v * angle.sin();
        let across = self.u * -dir.dot(self.v) + self.v * dir.dot(self.u);
        let across = across.normalize_or_zero() * CRACK_HALF_WIDTH;
        let start = self.centre;
        let end = self.centre + dir * CRACK_LENGTH;
        [start - across, end - across, end + across, start + across]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HALF: Vec3 = Vec3::new(2.2, 0.9, 0.7);

    #[test]
    fn an_undamaged_car_has_no_marks() {
        assert!(marks(&Damage::new(10), HALF).is_empty());
    }

    #[test]
    fn scraped_paint_adds_one_scuff_per_zone() {
        let mut damage = Damage::new(10);
        damage.impact(0, 1, false);
        damage.impact(2, 3, false);
        let scuffs = marks(&damage, HALF).iter().filter(|m| m.mark == Mark::Scuff).count();
        assert_eq!(scuffs, 2);
    }

    #[test]
    fn cracks_grow_with_the_glass_level_only_on_glazed_zones() {
        let mut damage = Damage::new(10);
        damage.impact(0, 2, true);
        let cracks = |d: &Damage| marks(d, HALF).iter().filter(|m| m.mark == Mark::Crack).count();
        assert_eq!(cracks(&damage), 2, "one glass level gives two cracks");
        damage.impact(0, 3, true);
        assert_eq!(cracks(&damage), 4);
        let mut roof = Damage::new(10);
        roof.impact(8, 6, true);
        assert_eq!(cracks(&roof), 0, "the roof has no glazing");
    }

    #[test]
    fn marks_lie_flat_on_one_face_of_the_body() {
        let mut damage = Damage::new(10);
        for zone in 0..10 {
            damage.impact(zone, 6, true);
        }
        for mark in marks(&damage, HALF) {
            let on_a_face =
                (0..3).any(|axis| mark.corners.iter().all(|c| (c[axis].abs() - (HALF[axis] + LIFT)).abs() < 1e-4));
            assert!(on_a_face, "{:?} is not on a face", mark.corners);
        }
    }

    #[test]
    fn a_front_scuff_sits_on_the_nose() {
        let mut damage = Damage::new(10);
        damage.impact(0, 1, false);
        let scuff = marks(&damage, HALF)[0];
        let centre = scuff.corners.iter().sum::<Vec3>() / 4.0;
        assert!((centre.x - (HALF.x + LIFT)).abs() < 1e-3, "{centre:?}");
    }
}
