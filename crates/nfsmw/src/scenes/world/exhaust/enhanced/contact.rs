//! Short, bounded world queries along the same centreline used by the flame shader.
//! The surface glow approximates local light spill; it is not a shadow-casting light.
use blackbox_collision::{BARRIER_TWO_SIDED, CollisionWorld, GROUP_EXCLUSION, Hit, HitKind, RayOptions};
use blackbox_gfx::{EffectVertex, FlameVolume};
use glam::{Vec2, Vec3};

use crate::scenes::world::space;

const SEGMENTS: usize = 6;
const GRID: usize = 5;

fn profile(height: f32) -> f32 {
    ((height - 0.35) / 0.65).max(0.0).powi(2)
}

fn centre(volume: &FlameVolume, height: f32) -> Vec3 {
    let bend = volume.appearance.bend;
    Vec3::new(bend.x * profile(height), height - 0.5, bend.z * profile(height))
}

fn cast(world: &CollisionWorld, from: Vec3, to: Vec3) -> Option<Hit> {
    let options = RayOptions { exclude: u32::from(GROUP_EXCLUSION), skip_groups: true, ..Default::default() };
    let hit = world.ray_cast_filtered(space::to_physics(from), space::to_physics(to), &options, |h| {
        h.kind != HitKind::Barrier || h.front_facing || h.surface_flags & BARRIER_TWO_SIDED != 0
    })?;
    // The collision API lengthens sub-centimetre rays. Never accept a hit beyond our segment.
    let delta = to - from;
    let t = (space::to_render(hit.point) - from).dot(delta) / delta.length_squared();
    if !(0.0..=1.0).contains(&t) {
        return None;
    }
    Some(Hit { t, ..hit })
}

fn nearest(world: &CollisionWorld, volume: &FlameVolume) -> Option<(Hit, f32)> {
    let radius = 0.7 / (3.4 * volume.appearance.radial_scale);
    let offsets = [Vec3::ZERO, Vec3::X * radius, Vec3::NEG_X * radius, Vec3::Z * radius, Vec3::NEG_Z * radius];
    for segment in 0..SEGMENTS {
        let from_h = segment as f32 * 1.1 / SEGMENTS as f32;
        let to_h = (segment + 1) as f32 * 1.1 / SEGMENTS as f32;
        let mut nearest: Option<(Hit, f32)> = None;
        for offset in offsets {
            let from = volume.to_world.transform_point3(centre(volume, from_h) + offset * from_h.min(1.0));
            let to = volume.to_world.transform_point3(centre(volume, to_h) + offset * to_h.min(1.0));
            let Some(hit) = cast(world, from, to) else { continue };
            let height = from_h + (to_h - from_h) * hit.t;
            if nearest.as_ref().is_none_or(|(_, h)| height < *h) {
                nearest = Some((hit, height));
            }
        }
        if nearest.is_some() {
            return nearest;
        }
    }
    None
}

pub(super) fn apply(
    world: &CollisionWorld,
    volume: &mut FlameVolume,
    glows: &mut Vec<EffectVertex>,
    root: [u8; 3],
    tail: [u8; 3],
) {
    let Some((hit, height)) = nearest(world, volume) else { return };
    let point = space::to_render(hit.point);
    let normal = space::to_render(hit.normal).normalize_or_zero();
    if !point.is_finite() || normal.length_squared() < 0.9 {
        return;
    }
    let plane = normal.extend(-normal.dot(point));
    // Keep metre distances after transforming the plane into the non-uniform proxy.
    volume.appearance.contact_plane = volume.to_world.transpose() * plane;
    let proximity = (1.0 - height / 1.1).max(0.0).sqrt();
    let flicker =
        0.88 + 0.08 * (volume.time * 23.0 + volume.seed).sin() + 0.04 * (volume.time * 41.0 + volume.seed * 3.0).sin();
    let mut colour = [255, 170, 80, 0];
    if volume.blue {
        let mix = ((height - 0.05) / 0.4).clamp(0.0, 1.0);
        for i in 0..3 {
            colour[i] = (f32::from(root[i]) * (1.0 - mix) + f32::from(tail[i]) * mix) as u8;
        }
    }
    colour[3] = (120.0 * volume.intensity * proximity * flicker).clamp(0.0, 120.0) as u8;
    if colour[3] == 0 {
        return;
    }
    let radius = (0.22 + volume.to_world.transform_vector3(Vec3::Y).length() * 0.25).clamp(0.25, 0.65);
    surface_glow(world, point, normal, radius, colour, glows);
}

fn surface_glow(
    world: &CollisionWorld,
    point: Vec3,
    normal: Vec3,
    radius: f32,
    colour: [u8; 4],
    out: &mut Vec<EffectVertex>,
) {
    let axis = match normal.z.abs() < 0.9 {
        true => Vec3::Z,
        false => Vec3::X,
    };
    let right = normal.cross(axis).normalize();
    let up = right.cross(normal);
    let mut vertices = [None; GRID * GRID];
    // Project each vertex, rather than floating a billboard through corners or over an edge.
    for y in 0..GRID {
        for x in 0..GRID {
            let uv = Vec2::new(x as f32, y as f32) / (GRID - 1) as f32;
            let p = point + (right * (uv.x * 2.0 - 1.0) + up * (uv.y * 2.0 - 1.0)) * radius;
            let Some(hit) = cast(world, p + normal * 0.08, p - normal * 0.08) else { continue };
            let n = space::to_render(hit.normal).normalize_or_zero();
            if n.dot(normal) < 0.85 {
                continue;
            }
            vertices[y * GRID + x] = Some(EffectVertex {
                position: (space::to_render(hit.point) + n * 0.006).to_array(),
                color: colour,
                uv: uv.to_array(),
                detail: [0.0; 2],
            });
        }
    }
    for y in 0..GRID - 1 {
        for x in 0..GRID - 1 {
            let a = y * GRID + x;
            for triangle in [[a, a + 1, a + GRID + 1], [a, a + GRID + 1, a + GRID]] {
                let [Some(a), Some(b), Some(c)] = triangle.map(|i| vertices[i]) else { continue };
                out.extend([a, b, c]);
            }
        }
    }
}
