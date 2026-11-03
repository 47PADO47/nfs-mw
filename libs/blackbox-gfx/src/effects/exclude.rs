//! Subtract an oriented owner's box from triangles, retaining interpolated mask coordinates.
use super::EffectVertex;
use glam::{Mat4, Vec3};
#[cfg(test)]
mod tests;

#[derive(Clone, Copy)]
struct Point {
    vertex: EffectVertex,
    local: Vec3,
}

impl Point {
    fn lerp(self, other: Self, t: f32) -> Self {
        Self {
            local: self.local.lerp(other.local, t),
            vertex: EffectVertex {
                position: Vec3::from(self.vertex.position).lerp(Vec3::from(other.vertex.position), t).to_array(),
                color: std::array::from_fn(|i| {
                    (f32::from(self.vertex.color[i]) * (1.0 - t) + f32::from(other.vertex.color[i]) * t).round() as u8
                }),
                uv: std::array::from_fn(|i| self.vertex.uv[i] * (1.0 - t) + other.vertex.uv[i] * t),
                detail: std::array::from_fn(|i| self.vertex.detail[i] * (1.0 - t) + other.vertex.detail[i] * t),
            },
        }
    }
}

fn split(poly: &[Point], axis: usize, edge: f32, sign: f32) -> (Vec<Point>, Vec<Point>) {
    let (mut inside, mut outside) = (Vec::new(), Vec::new());
    let Some(&last) = poly.last() else { return (inside, outside) };
    let mut a = last;
    let mut da = (a.local[axis] - edge) * sign;
    for &b in poly {
        let db = (b.local[axis] - edge) * sign;
        if (da >= 0.0) != (db >= 0.0) {
            let hit = a.lerp(b, da / (da - db));
            inside.push(hit);
            outside.push(hit);
        }
        match db >= 0.0 {
            true => inside.push(b),
            false => outside.push(b),
        }
        a = b;
        da = db;
    }
    (inside, outside)
}

fn emit(poly: &[Point], out: &mut Vec<EffectVertex>) {
    for i in 1..poly.len().saturating_sub(1) {
        out.extend([poly[0].vertex, poly[i].vertex, poly[i + 1].vertex]);
    }
}

impl super::EffectLayer {
    /// Only vertices from `start` onward are clipped. Most exterior triangles take a fast path.
    pub fn exclude_box(out: &mut Vec<EffectVertex>, start: usize, inverse: Mat4, min: Vec3, max: Vec3) {
        if start > out.len() || !inverse.is_finite() || !min.is_finite() || !max.is_finite() {
            return;
        }
        let source = out.split_off(start);
        for triangle in source.as_chunks::<3>().0 {
            let points: [Point; 3] = std::array::from_fn(|i| Point {
                vertex: triangle[i],
                local: inverse.transform_point3(Vec3::from(triangle[i].position)),
            });
            if (0..3).any(|axis| {
                points.iter().all(|p| p.local[axis] < min[axis]) || points.iter().all(|p| p.local[axis] > max[axis])
            }) {
                out.extend_from_slice(triangle);
                continue;
            }
            let mut remainder = points.to_vec();
            for (axis, edge, sign) in [
                (0, min.x, 1.0),
                (0, max.x, -1.0),
                (1, min.y, 1.0),
                (1, max.y, -1.0),
                (2, min.z, 1.0),
                (2, max.z, -1.0),
            ] {
                let (inside, exterior) = split(&remainder, axis, edge, sign);
                emit(&exterior, out);
                remainder = inside;
                if remainder.len() < 3 {
                    break;
                }
            }
        }
    }
}
