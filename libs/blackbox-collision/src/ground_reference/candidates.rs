use std::collections::BTreeSet;

use super::{GroundFace, GroundReferenceSettings, geometry};
use crate::math::{Vec3, sub};
use crate::{CollisionWorld, InstanceRef};

pub(super) fn collect(
    world: &CollisionWorld,
    center: Vec3,
    radius: f32,
    settings: &GroundReferenceSettings,
    mut group_enabled: impl FnMut(u16) -> bool,
) -> Vec<GroundFace> {
    let mut result = Vec::new();
    for (section, index) in references(world, center, radius, settings.maximum_cell_span) {
        let Some(pack) = world.pack(section) else { continue };
        let (Some(instance), Some(article)) = (pack.instances.get(index), pack.article_of(index)) else { continue };
        if instance.flags & settings.instance_exclusion != 0 || (instance.group != 0 && !group_enabled(instance.group))
        {
            continue;
        }
        let point = instance.position();
        let distance = (point[0] - center[0]).powi(2) + (point[2] - center[2]).powi(2);
        if distance >= (instance.broad_radius() + radius).powi(2) {
            continue;
        }
        let local = instance.to_local(center);
        for strip in &article.strips {
            let distance = (strip.center[0] - local[0]).powi(2) + (strip.center[2] - local[2]).powi(2);
            if distance >= (strip.radius + radius).powi(2) {
                continue;
            }
            for mut triangle in strip.triangles() {
                triangle.pts = triangle.pts.map(|p| sub(p, instance.translation));
                if !geometry::overlaps(triangle.pts, center, radius, settings.edge_length_epsilon) {
                    continue;
                }
                result.push(GroundFace {
                    triangle,
                    surface_hash: article.surface_hash(triangle.surface).unwrap_or(0),
                    section,
                    instance: index,
                });
            }
        }
    }
    result
}

pub(super) fn references(world: &CollisionWorld, center: Vec3, radius: f32, maximum_span: u32) -> Vec<(u32, usize)> {
    let Some(grid) = world.grid() else {
        return world.packs().flat_map(|pack| (0..pack.instances.len()).map(|i| (pack.section, i))).collect();
    };
    let coordinate = |value: f32, minimum: f32, count: u32| {
        (((value - minimum) / grid.edge).trunc().max(0.0) as u32).min(count.saturating_sub(1))
    };
    let r0 = coordinate(center[2] - radius, grid.min[2], grid.rows);
    let c0 = coordinate(center[0] - radius, grid.min[0], grid.cols);
    let mut r1 = coordinate(center[2] + radius, grid.min[2], grid.rows);
    let mut c1 = coordinate(center[0] + radius, grid.min[0], grid.cols);
    if r1 - r0 > maximum_span {
        r1 = r0;
    }
    if c1 - c0 > maximum_span {
        c1 = c0;
    }
    let mut seen = BTreeSet::<InstanceRef>::new();
    let mut result = Vec::new();
    for col in c0..=c1 {
        for row in r0..=r1 {
            let Some(node) = grid.node(grid.node_index(row, col)) else { continue };
            for &reference in &node.instances {
                if seen.insert(reference) {
                    result.push((reference.section(), reference.index()));
                }
            }
        }
    }
    result
}
