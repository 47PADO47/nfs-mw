//! Conservative lens dimensions from named installed brake-lamp solids.
use super::*;
const FALLBACK: Vec3 = Vec3::new(0.0, 0.20, 0.11);
pub(super) fn collect(model: &CarModel, lamps: &[Lamp]) -> Vec<(Vec3, Vec3)> {
    lamps
        .iter()
        .filter(|l| l.kind == Kind::Tail)
        .map(|lamp| {
            let mut min = Vec3::splat(f32::INFINITY);
            let mut max = Vec3::splat(f32::NEG_INFINITY);
            let mut count = 0;
            for p in &model.placements {
                let Some(solid) = model.solids.get(&p.solid) else { continue };
                if !solid.name.contains("BRAKELIGHT") {
                    continue;
                }
                for v in &solid.vertices {
                    let at = p.transform.transform_point3(Vec3::from_array(v.position));
                    if !at.is_finite() || at.distance(lamp.at) > 0.55 {
                        continue;
                    }
                    min = min.min(at);
                    max = max.max(at);
                    count += 1;
                }
            }
            let span = max - min;
            let dimensions = match count >= 3 && span.y > 0.03 && span.z > 0.02 {
                true => Vec3::new(0.0, span.y.clamp(0.08, 0.4), span.z.clamp(0.06, 0.25)),
                false => FALLBACK,
            };
            (lamp.at, dimensions)
        })
        .collect()
}
impl CarLights {
    pub fn aperture(&self, at: Vec3) -> Vec3 {
        self.apertures
            .iter()
            .find(|(position, _)| position.distance_squared(at) < 1e-6)
            .map_or(FALLBACK, |(_, span)| *span)
    }
}
