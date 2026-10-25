//! Stable, material-independent selection for a bounded reflection texture cache.
use super::mesh::Mesh;
use blackbox_gfx::Instance;
use glam::Vec3;
use std::collections::{HashMap, HashSet};

pub(super) fn budget(quality: u32) -> (u32, u32) {
    match quality {
        0 => (256, 128),
        1 => (512, 192),
        2 => (1024, 192),
        _ => (1024, 384),
    }
}
fn score(distance: f32, radius: f32, resolution: u32, resident: bool) -> f32 {
    // Bound the size bonus so a huge terrain mesh cannot monopolize the cache.
    let extent = radius.clamp(1.0, 30.0);
    let proximity = extent / (distance.max(0.0) + 10.0);
    let detail = (resolution as f32 / 128.0).log2().clamp(1.0, 4.0);
    proximity
        * detail
        * match resident {
            true => 1.3,
            false => 1.0,
        }
}
fn ranked(scores: HashMap<usize, f32>, capacity: usize) -> HashSet<usize> {
    let mut entries: Vec<_> = scores.into_iter().collect();
    entries.sort_unstable_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
    entries.into_iter().take(capacity).map(|(id, _)| id).collect()
}
pub(super) fn select<'a>(
    meshes: &HashMap<usize, Mesh>,
    instances: impl Iterator<Item = &'a Instance>,
    views: &HashMap<usize, wgpu::TextureView>,
    camera: Vec3,
    capacity: usize,
    resident: &HashSet<usize>,
) -> HashSet<usize> {
    let mut scores = HashMap::<usize, f32>::new();
    for instance in instances {
        let Some(mesh) = meshes.get(&instance.mesh.raw()) else { continue };
        let center = instance.transform.transform_point3(mesh.center);
        let scale = instance
            .transform
            .x_axis
            .truncate()
            .length()
            .max(instance.transform.y_axis.truncate().length())
            .max(instance.transform.z_axis.truncate().length());
        let radius = mesh.radius * scale;
        let distance = (center.distance(camera) - radius).max(0.0);
        for &handle in &mesh.textures {
            let Some(view) = views.get(&handle) else { continue };
            let resolution = view.texture().width().max(view.texture().height());
            if resolution <= 128 {
                continue;
            }
            let value = score(distance, radius, resolution, resident.contains(&handle));
            scores.entry(handle).and_modify(|s| *s = s.max(value)).or_insert(value);
        }
    }
    ranked(scores, capacity)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn budgets_are_bounded_and_increase_monotonically() {
        let mut last = 0;
        for quality in 0..4 {
            let (size, count) = budget(quality);
            let bytes = u64::from(size).pow(2) * u64::from(count) * 4;
            assert!(bytes > last && count < 32768);
            last = bytes;
        }
    }
    #[test]
    fn nearby_detail_wins_without_material_or_part_names() {
        let scores = [(1, score(5.0, 3.0, 1024, false)), (2, score(200.0, 3.0, 1024, true))].into();
        assert_eq!(ranked(scores, 1), HashSet::from([1]));
    }
    #[test]
    fn stable_ties_and_retention_prevent_small_distance_churn() {
        assert!(score(11.0, 2.0, 512, true) > score(10.0, 2.0, 512, false));
        assert_eq!(ranked([(2, 1.0), (1, 1.0), (3, 1.0)].into(), 2), HashSet::from([1, 2]));
        assert!(ranked([(1, 1.0)].into(), 0).is_empty());
    }
}
