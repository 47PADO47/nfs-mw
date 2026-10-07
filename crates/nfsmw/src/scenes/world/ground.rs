//! A rough ground height, until collision data is loaded: the median bottom of
//! the scenery boxes around a point (roads and terrain dominate the count).

use super::resident::Placed;

pub fn height_near<'a>(placed: impl Iterator<Item = &'a Placed>, x: f32, y: f32, radius: f32) -> Option<f32> {
    let mut bottoms: Vec<f32> = placed
        .filter(|p| {
            let c = p.bounds.center();
            (c.x - x).abs() < radius && (c.y - y).abs() < radius
        })
        .map(|p| p.bounds.min.z)
        .collect();
    if bottoms.is_empty() {
        return None;
    }
    let mid = bottoms.len() / 2;
    Some(*bottoms.select_nth_unstable_by(mid, f32::total_cmp).1)
}
