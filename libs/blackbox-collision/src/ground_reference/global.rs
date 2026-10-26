//! Resident instance point search used when no prepared collider list is supplied.
use super::{GroundFace, GroundPlane, GroundReferenceSettings, candidates, geometry};
use crate::math::{Vec3, segment_point_dist_sq};
use crate::{CollisionWorld, STRIP_FACING_UNKNOWN, STRIP_UP_FACING, Triangle};

/// Geometric policy for upright/tilted point searches, supplied by the game adapter.
#[derive(Clone, Copy, Debug)]
pub struct PointQuerySettings {
    pub cache: GroundReferenceSettings,
    pub minimum_face_offset: f32,
    pub unknown_winding_height: f32,
    pub downward_distance: f32,
    pub upright_face_exclusion: u8,
}

pub(super) fn update(
    face: &mut Option<GroundFace>,
    plane: &mut GroundPlane,
    world: &CollisionWorld,
    position: Vec3,
    settings: &PointQuerySettings,
    group_enabled: impl FnMut(u16) -> bool,
) -> GroundPlane {
    let same = plane.valid && face.is_some_and(|f| geometry::contains(f.triangle.pts, position));
    if same && settings.cache.reuse_face {
        plane.separation = geometry::separation(face.unwrap().triangle.pts[0], position, plane.normal);
        return *plane;
    }
    let was_valid = plane.valid;
    let selected = search(world, position, settings, group_enabled);
    plane.valid = selected.is_some();
    plane.surface_hash = selected.map_or(0, |f| f.surface_hash);
    if let Some(selected) = selected {
        *face = Some(selected);
    }
    plane.valid |= was_valid && settings.cache.preserve_face;
    if !plane.valid {
        plane.separation = -1.0;
        return *plane;
    }
    let Some(face) = *face else { return *plane };
    plane.normal = geometry::normal(face.triangle.pts, settings.cache.maximum_normal_y);
    plane.separation = geometry::separation(face.triangle.pts[0], position, plane.normal);
    *plane
}

fn search(
    world: &CollisionWorld,
    position: Vec3,
    settings: &PointQuerySettings,
    mut group_enabled: impl FnMut(u16) -> bool,
) -> Option<GroundFace> {
    let cache = &settings.cache;
    let query = [position[0], position[1] + cache.vertical_tolerance, position[2]];
    let mut best = f32::INFINITY;
    let mut result = None;
    for (section, index) in candidates::references(world, query, 0.0, cache.maximum_cell_span) {
        let Some(pack) = world.pack(section) else { continue };
        let (Some(instance), Some(article)) = (pack.instances.get(index), pack.article_of(index)) else { continue };
        if instance.flags & cache.instance_exclusion != 0 || (instance.group != 0 && !group_enabled(instance.group)) {
            continue;
        }
        let center = instance.position();
        if (center[0] - query[0]).powi(2) + (center[2] - query[2]).powi(2) >= instance.broad_radius().powi(2) {
            continue;
        }
        let local = instance.to_local(query);
        let tilted = instance.needs_cross();
        if !tilted && (local[0].abs() > instance.half_width || local[2].abs() > instance.half_length) {
            continue;
        }
        let start = [query[0], query[1] + cache.vertical_tolerance, query[2]];
        let end = [query[0], query[1] - settings.downward_distance, query[2]];
        for strip in &article.strips {
            if strip.flags & cache.instance_exclusion != 0 {
                continue;
            }
            let mut selected = None;
            let mut gap = f32::INFINITY;
            if tilted {
                if segment_point_dist_sq(instance.to_local(start), instance.to_local(end), strip.center)
                    >= strip.radius.powi(2)
                {
                    continue;
                }
                for triangle in strip.triangles() {
                    if u16::from(triangle.flags) & cache.instance_exclusion != 0 {
                        continue;
                    }
                    let world_pts = triangle.pts.map(|p| instance.to_world(p));
                    if !geometry::contains(world_pts, start) {
                        continue;
                    }
                    let n = geometry::normal(world_pts, cache.maximum_normal_y);
                    let distance = start[1] - geometry::height(world_pts[0], n, start);
                    if distance <= cache.minimum_vertical_separation || distance >= gap {
                        continue;
                    }
                    gap = distance;
                    selected = Some(Triangle { pts: world_pts, ..triangle });
                }
            }
            if !tilted {
                if (strip.center[0] - local[0]).powi(2) + (strip.center[2] - local[2]).powi(2) >= strip.radius.powi(2) {
                    continue;
                }
                for (i, triangle) in strip.triangles().enumerate() {
                    if triangle.flags & settings.upright_face_exclusion != 0
                        || u16::from(triangle.flags) & cache.instance_exclusion != 0
                    {
                        continue;
                    }
                    let unknown = strip.flags & STRIP_FACING_UNKNOWN != 0;
                    let right = (strip.flags & STRIP_UP_FACING != 0) ^ (i % 2 != 0);
                    let contains = match unknown {
                        true => geometry::contains(triangle.pts, local),
                        false => geometry::contains_winding(triangle.pts, local, right),
                    };
                    if !contains {
                        continue;
                    }
                    let minimum_y = triangle.pts.iter().map(|p| p[1]).fold(f32::INFINITY, f32::min);
                    if unknown && minimum_y >= local[1] + settings.unknown_winding_height {
                        continue;
                    }
                    let n = geometry::normal(triangle.pts, cache.maximum_normal_y);
                    gap = local[1] - geometry::height(triangle.pts[0], n, local);
                    if minimum_y - settings.minimum_face_offset < local[1] {
                        selected = Some(Triangle { pts: triangle.pts.map(|p| instance.to_world(p)), ..triangle });
                    }
                    break; // The strip's first eligible containing face is authoritative.
                }
            }
            if gap <= 0.0 || selected.is_none() {
                continue;
            }
            let distance = gap - f32::from(u8::from(tilted)) * cache.vertical_tolerance;
            if distance >= best {
                continue;
            }
            best = distance;
            let triangle = selected.unwrap();
            result = Some(GroundFace {
                triangle,
                surface_hash: article.surface_hash(triangle.surface).unwrap_or(0),
                section,
                instance: index,
            });
        }
    }
    result
}
