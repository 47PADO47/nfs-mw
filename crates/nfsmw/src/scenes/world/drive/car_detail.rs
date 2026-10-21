//! Main-view projected car geometry. Source and gaps: docs/specs/car-detail.md.
use crate::settings::{CarGeometry, DetailSettings};
use blackbox_scenery::LodView;
use glam::Vec3;
use nfsmw_data::car::CarModel;

#[derive(Clone, Copy)]
pub struct DetailView {
    pub view: LodView,
    pub settings: DetailSettings,
}
pub struct DrawOptions<'a> {
    pub car: u32,
    pub lamps: [bool; 2],
    pub detail: Option<&'a DetailView>,
    pub glass: u16,
    pub broken_lights: u32,
    pub damage: Option<&'a [u8; 10]>,
}
impl Default for DrawOptions<'_> {
    fn default() -> Self {
        Self { car: 0, lamps: [true, false], detail: None, glass: 0, broken_lights: 0, damage: None }
    }
}

pub(super) fn choose(model: &CarModel, position: Vec3, radius: f32, view: &DetailView, player: bool) -> usize {
    if model.driving_lods.levels.is_empty() {
        return 0;
    }
    if view.settings.car == CarGeometry::Highest || (player && view.settings.highest_player) {
        return 0;
    }
    let size = projected_size(&view.view, position, radius) as f32;
    let minimum = usize::from(view.settings.car == CarGeometry::Low);
    let wanted = index(size, model.driving_lods.traffic, minimum);
    available(model, wanted)
}

pub(super) fn radius(model: &CarModel) -> f32 {
    let mut min = Vec3::splat(f32::INFINITY);
    let mut max = Vec3::splat(f32::NEG_INFINITY);
    for placement in &model.placements {
        if placement.corner.is_some() {
            continue;
        }
        let Some(solid) = model.solids.get(&placement.solid) else { continue };
        min = min.min(Vec3::from_array(solid.bounds_min));
        max = max.max(Vec3::from_array(solid.bounds_max));
    }
    ((max - min).length() * 0.5).clamp(0.1, 100.0)
}

fn index(size: f32, traffic: bool, minimum: usize) -> usize {
    let thresholds = match traffic {
        true => [80.0, 40.0, 20.0, 10.0],
        false => [120.0, 25.0, 20.0, 10.0],
    };
    (minimum + thresholds.iter().take_while(|&&threshold| size < threshold).count()).min(4)
}

fn projected_size(view: &LodView, position: Vec3, radius: f32) -> i32 {
    let to = position - Vec3::from_array(view.position);
    if to.dot(Vec3::from_array(view.forward)) < -radius {
        return 0;
    }
    let away = to.length() - radius;
    if away <= radius * 0.5 {
        return (view.pixel_scale * 2.0) as i32;
    }
    (radius * view.pixel_scale / away) as i32
}
fn available(model: &CarModel, wanted: usize) -> usize {
    model.driving_lods.levels.iter().map(|l| l.level).chain([0]).filter(|&level| level <= wanted).max().unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pc_traffic_thresholds_and_boundaries_differ_from_ordinary_cars() {
        assert_eq!(index(120.0, false, 0), 0);
        assert_eq!(index(119.0, false, 0), 1);
        assert_eq!(index(80.0, true, 0), 0);
        assert_eq!(index(79.0, true, 0), 1);
        assert_eq!(index(40.0, true, 0), 1);
        assert_eq!(index(39.0, true, 0), 2);
        assert_eq!(index(39.0, false, 0), 1);
        assert_eq!(index(200.0, false, 1), 1);
        assert_eq!(index(0.0, false, 1), 4);
    }

    #[test]
    fn car_projection_uses_its_close_range_diameter_rule() {
        let view = LodView { position: [0.0; 3], forward: [1.0, 0.0, 0.0], pixel_scale: 343.0 };
        assert_eq!(projected_size(&view, Vec3::X, 2.0), 686);
        assert_eq!(projected_size(&view, Vec3::X * 3.0, 2.0), 686);
        assert_eq!(projected_size(&view, Vec3::X * 6.0, 2.0), 171);
        assert_eq!(projected_size(&view, Vec3::NEG_X * 6.0, 2.0), 0);
    }
}
