//! Seating a spike strip on the road. The strip is 6.5 m across, so the ground under its centre alone leaves an end
//! below the surface on a banked or sloping road. The ground is read at both ends as well and the strip is tilted
//! onto the line through them. Spec: `docs/specs/ai-pursuit-roadblocks.md` section 2.5 (the original reads one ray at
//! the position; this seating is the rewrite's).

use glam::Vec3;

/// A centre this much above the mean of the two ends is a crowned road: the strip stands level at the centre height,
/// so neither end can sink and the road's own camber is left alone (metres).
const CROWN: f32 = 0.05;

/// The height and the upward normal for a strip whose centre ground is `(height, normal)` and whose ends, a half
/// width either side along `across` (a unit horizontal vector, physics space), stand on `ends` (the ground height under
/// each end, `across` side first). A missing end keeps the centre reading.
pub fn seat(centre: (f32, Vec3), across: Vec3, half: f32, ends: [Option<f32>; 2]) -> (f32, Vec3) {
    let (height, normal) = centre;
    let (Some(plus), Some(minus)) = (ends[0], ends[1]) else {
        return (height, normal);
    };
    let mean = (plus + minus) * 0.5;
    if height > mean + CROWN {
        return (height, normal);
    }
    let slope = (plus - minus) / (2.0 * half.max(1e-3));
    let flat = normal - across * normal.dot(across);
    let tilted = (flat - across * slope * flat.y).try_normalize().unwrap_or(normal);
    (mean.max(height), tilted)
}

#[cfg(test)]
mod tests {
    use super::*;

    const ACROSS: Vec3 = Vec3::Z;

    #[test]
    fn a_level_road_keeps_the_centre_reading() {
        let (height, normal) = seat((2.0, Vec3::Y), ACROSS, 3.0, [Some(2.0), Some(2.0)]);
        assert_eq!(height, 2.0);
        assert!((normal - Vec3::Y).length() < 1e-6);
    }

    #[test]
    fn a_banked_road_tilts_the_strip_onto_both_ends() {
        // Rising 0.6 m over the 6 m between the ends toward +across: the centre sits at the mean.
        let (height, normal) = seat((2.0, Vec3::Y), ACROSS, 3.0, [Some(2.3), Some(1.7)]);
        assert!((height - 2.0).abs() < 1e-6);
        let slope = -normal.dot(ACROSS) / normal.y;
        assert!((slope - 0.1).abs() < 1e-5, "the strip rises 0.1 m per metre across: {slope}");
        assert!(normal.is_normalized());
    }

    #[test]
    fn a_centre_reading_below_the_ends_is_lifted_to_their_mean() {
        let (height, _) = seat((1.9, Vec3::Y), ACROSS, 3.0, [Some(2.3), Some(2.3)]);
        assert!((height - 2.3).abs() < 1e-6);
    }

    #[test]
    fn a_crowned_road_stays_level_at_the_centre() {
        let (height, normal) = seat((2.2, Vec3::Y), ACROSS, 3.0, [Some(2.0), Some(2.0)]);
        assert_eq!(height, 2.2);
        assert!((normal - Vec3::Y).length() < 1e-6);
    }

    #[test]
    fn a_missing_end_keeps_the_centre_reading() {
        let normal = Vec3::new(0.1, 1.0, 0.0).normalize();
        assert_eq!(seat((2.0, normal), ACROSS, 3.0, [Some(2.5), None]), (2.0, normal));
    }
}
