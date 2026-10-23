//! Authored enclosure-bound PC rain volume, distinct from primary camera rain.
use super::drops::random;
use blackbox_roads::TrackZone;
use glam::{Vec2, Vec3};

#[derive(Debug, Clone)]
pub struct Curtain {
    positions: Vec<Vec3>,
    seed: u32,
    edge: Option<[Vec2; 2]>,
    outward: Vec2,
    count: usize,
}
impl Default for Curtain {
    fn default() -> Self {
        Self { positions: vec![Vec3::ZERO; 200], seed: 0x7241_A7E3, edge: None, outward: Vec2::ZERO, count: 0 }
    }
}
fn edges(zone: &TrackZone) -> impl Iterator<Item = [Vec2; 2]> + '_ {
    zone.polygon.iter().copied().zip(zone.polygon.iter().copied().cycle().skip(1)).map(|(a, b)| [a, b])
}
fn nearest_edge(zone: &TrackZone, point: Vec2) -> Option<[Vec2; 2]> {
    edges(zone)
        .filter_map(|edge| {
            let normal = (edge[1] - edge[0]).perp().try_normalize()?;
            let distance = (edge[0] - point).dot(normal);
            let positive = point + normal * distance * 1.1;
            let negative = point - normal * distance * 1.1;
            if !zone.contains(positive) && !zone.contains(negative) {
                return None;
            }
            Some((distance.abs(), edge))
        })
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, edge)| edge)
}
impl Curtain {
    pub fn update(&mut self, dt: f32, camera: Vec3, forward: Vec3, zone: Option<&TrackZone>, intensity: f32) {
        let Some(zone) = zone.filter(|z| (3..6).contains(&z.polygon.len())) else {
            self.edge = None;
            self.count = 0;
            return;
        };
        let direction = forward.truncate().normalize_or(Vec2::X);
        let Some(first) = nearest_edge(zone, camera.truncate() + direction * 10.0) else {
            self.count = 0;
            return;
        };
        let axis = (first[1] - first[0]).normalize_or(Vec2::X);
        let opposite = edges(zone)
            .filter(|edge| *edge != first)
            .max_by(|a, b| {
                let alignment = |e: [Vec2; 2]| (e[1] - e[0]).normalize_or_zero().dot(axis).abs();
                alignment(*a).total_cmp(&alignment(*b))
            })
            .unwrap_or(first);
        let facing = |e: [Vec2; 2]| ((e[0] + e[1]) * 0.5 - camera.truncate()).dot(direction);
        let edge = match facing(first) >= facing(opposite) {
            true => first,
            false => opposite,
        };
        self.outward = ((edge[0] + edge[1] - zone.bbox_min - zone.bbox_max) * 0.5).normalize_or(Vec2::X);
        let changed = self.edge != Some(edge);
        self.edge = Some(edge);
        self.count = (intensity.clamp(0.0, 1.0) * 200.0) as usize;
        for p in &mut self.positions {
            p.z -= 21.9 * dt;
            if changed || p.z < camera.z || p.z > camera.z + 10.0 {
                let along = random(&mut self.seed);
                let outside = random(&mut self.seed) * 10.0;
                let height = random(&mut self.seed) * 10.0;
                *p = (edge[0].lerp(edge[1], along) + self.outward * outside).extend(camera.z + height);
            }
        }
    }
    pub fn quads(&self, camera: Vec3, out: &mut Vec<([Vec3; 4], usize)>) {
        for (i, &at) in self.positions.iter().take(self.count).enumerate() {
            let axis = Vec3::Z * 0.225;
            let side = (camera - at).cross(axis).normalize_or(Vec3::X) * 0.01;
            out.push(([at - axis - side, at - axis + side, at + axis + side, at + axis - side], i % 10));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn a_sheltered_camera_sees_rain_beyond_the_selected_boundary() {
        let zone = TrackZone {
            kind: 3,
            position: Vec2::ZERO,
            direction: Vec2::X,
            elevation: 10.0,
            visit_info: 0,
            data: [0; 4],
            bbox_min: Vec2::new(-20.0, -5.0),
            bbox_max: Vec2::new(20.0, 5.0),
            polygon: vec![Vec2::new(-20.0, -5.0), Vec2::new(20.0, -5.0), Vec2::new(20.0, 5.0), Vec2::new(-20.0, 5.0)],
        };
        let mut curtain = Curtain::default();
        let camera = Vec3::new(15.0, 0.0, 1.0);
        curtain.update(1.0 / 60.0, camera, Vec3::X, Some(&zone), 1.0);
        let mut quads = Vec::new();
        curtain.quads(camera, &mut quads);
        assert_eq!(quads.len(), 200);
        assert!(quads.iter().all(|(q, _)| q.iter().all(|p| p.x > 19.9)));
        curtain.update(1.0 / 60.0, camera, Vec3::X, None, 1.0);
        quads.clear();
        curtain.quads(camera, &mut quads);
        assert!(quads.is_empty());
    }
}
