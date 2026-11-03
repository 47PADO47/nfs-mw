//! Bounded Hermite reconstruction, then uniform arc-distance sampling.
use glam::Vec3;

pub(super) fn resample(points: &[Vec3]) -> Vec<Vec3> {
    if points.len() < 3 {
        return points.to_vec();
    }
    let mut dense = vec![points[0]];
    for i in 0..points.len() - 1 {
        let (a, b) = (points[i], points[i + 1]);
        let length = a.distance(b);
        let before = points[i.saturating_sub(1)];
        let after = points[(i + 2).min(points.len() - 1)];
        let m0 = ((b - before) * 0.5).clamp_length_max(length);
        let m1 = ((after - a) * 0.5).clamp_length_max(length);
        for j in 1..=4 {
            let t = j as f32 / 4.0;
            let t2 = t * t;
            let t3 = t2 * t;
            dense.push(
                (2.0 * t3 - 3.0 * t2 + 1.0) * a
                    + (t3 - 2.0 * t2 + t) * m0
                    + (-2.0 * t3 + 3.0 * t2) * b
                    + (t3 - t2) * m1,
            );
        }
    }
    let length: f32 = dense.windows(2).map(|p| p[0].distance(p[1])).sum();
    let step = (length / 48.0).max(0.08);
    let mut out = vec![points[0]];
    let mut travelled = 0.0;
    let mut next = step;
    for pair in dense.windows(2) {
        let segment = pair[0].distance(pair[1]);
        while next <= travelled + segment && out.len() < 64 {
            out.push(pair[0].lerp(pair[1], (next - travelled) / segment.max(1e-6)));
            next += step;
        }
        travelled += segment;
    }
    if out.last().unwrap().distance_squared(*points.last().unwrap()) > 1e-8 {
        out.push(*points.last().unwrap());
    }
    out
}
