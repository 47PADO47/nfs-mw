//! Small travelling variation in an otherwise historical path. The lamp root stays fixed.
use glam::Vec3;

pub(super) fn apply(points: &mut [Vec3], time: f32, amount: u16, width: f32, seed: f32) {
    if amount == 0 || points.len() < 2 {
        return;
    }
    let amplitude = width * 0.045 * f32::from(amount.min(200)) / 100.0;
    let mut distance = 0.0;
    let mut previous = points[0];
    for target in points.iter_mut().skip(1) {
        let point = *target;
        distance += point.distance(previous);
        let tangent = (point - previous).normalize_or(Vec3::X);
        let side = tangent.cross(Vec3::Z).normalize_or(Vec3::Y);
        let root = (distance / 0.4).clamp(0.0, 1.0);
        let phase = time * 12.0 - distance * 2.0 + seed;
        let wave = phase.sin() * 0.65 + (phase * 1.71 + 1.3).sin() * 0.35;
        *target += (side * wave + Vec3::Z * (phase * 0.83).cos() * 0.3) * amplitude * root * root;
        previous = point;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn trail_shimmer_is_bounded_anchored_and_repeatable() {
        let base: Vec<_> = (0..=60).map(|i| Vec3::X * i as f32 * 0.1).collect();
        let mut a = base.clone();
        let mut b = base.clone();
        apply(&mut a, 1.0, 200, 0.1, 1.3);
        apply(&mut b, 1.0, 200, 0.1, 1.3);
        assert_eq!(a, b);
        assert_eq!(a[0], base[0]);
        assert!(a.iter().zip(&base).all(|(a, b)| a.distance(*b) < 0.05));
        apply(&mut b, 1.1, 200, 0.1, 1.3);
        assert_ne!(a, b);
        let mut off = base.clone();
        apply(&mut off, 1.0, 0, 0.1, 1.3);
        assert_eq!(off, base);
    }
}
