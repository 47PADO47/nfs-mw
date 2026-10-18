//! Adapter between a right-handed scene camera and the supplied perspective fit.
use super::{
    fit::{Fit, Params},
    projected_fit,
};
use blackbox_gfx::{FrameParams, Projection, RenderExperiments};
use bytemuck::Zeroable;
use glam::{Mat4, Vec3};

pub(super) fn fit(
    frame: &FrameParams,
    settings: RenderExperiments,
    casters: &[(usize, Vec3, f32)],
    points: &mut Vec<Vec3>,
) -> Option<Fit> {
    let parameters = settings.shadow_projection?;
    let Projection::PerspectiveInfiniteReverse { fov_y, aspect, .. } = frame.projection else { return None };
    let [near, far] = parameters.frustum;
    if !near.is_finite() || !far.is_finite() || near <= 0.0 || far <= near {
        return None;
    }
    let inverse = frame.view.inverse();
    points.clear();
    for z in [near, far] {
        let height = z * (fov_y * 0.5).tan();
        for x in [-1.0, 1.0] {
            for y in [-1.0, 1.0] {
                points.push(inverse.transform_point3(Vec3::new(x * height * aspect, y * height, -z)));
            }
        }
    }
    let source_view = Mat4::from_scale(Vec3::new(1.0, 1.0, -1.0)) * frame.view;
    let sun = (-frame.light_dir).try_normalize()?;
    append_casters(points, casters, sun);
    for point in points.iter_mut() {
        *point = source_view.transform_point3(*point);
    }
    let fit = projected_fit::fit_view_points(source_view, source_view.transform_vector3(sun), points, &parameters)?;
    let mut params = Params::zeroed();
    params.matrices[0] = fit.clip_from_world.to_cols_array();
    params.splits = [far; 4];
    params.options = [1.0, settings.shadow_resolution as f32, 0.0, 0.0];
    params.resolutions = [settings.shadow_resolution as f32; 4];
    params.camera = frame.camera_position.extend(1.0).to_array();
    params.forward = inverse.transform_vector3(Vec3::NEG_Z).extend(0.0).to_array();
    params.sun = sun.extend(0.0).to_array();
    let strength = frame.surface_light.map_or(1.0, |light| light.shadow_strength).clamp(0.0, 1.0);
    params.projected = [1.0, 1.0 - strength, 0.0, projected_fit::texture_bias(settings.shadow_resolution)?.w_axis.x];
    // Original depth precision/raster bias remain separate from the exact fit.
    params.depth_spans[0] = fit.far - fit.near;
    params.texels[0] = fit.extent.x / settings.shadow_resolution as f32;
    Some(Fit { params, count: 1 })
}

fn append_casters(points: &mut Vec<Vec3>, casters: &[(usize, Vec3, f32)], sun: Vec3) {
    let right = sun.cross(Vec3::Z).try_normalize().unwrap_or(Vec3::X);
    let up = sun.cross(right);
    let mut lo = Vec3::splat(f32::INFINITY);
    let mut hi = Vec3::splat(f32::NEG_INFINITY);
    for &point in points.iter() {
        let p = Vec3::new(right.dot(point), up.dot(point), sun.dot(point));
        lo = lo.min(p);
        hi = hi.max(p);
    }
    for &(_, center, radius) in casters {
        if !center.is_finite() || !radius.is_finite() || radius < 0.0 {
            continue;
        }
        let p = Vec3::new(right.dot(center), up.dot(center), sun.dot(center));
        if p.x + radius < lo.x
            || p.x - radius > hi.x
            || p.y + radius < lo.y
            || p.y - radius > hi.y
            || p.z + radius < lo.z
        {
            continue;
        }
        for x in [-radius, radius] {
            for y in [-radius, radius] {
                for z in [-radius, radius] {
                    points.push(center + Vec3::new(x, y, z));
                }
            }
        }
    }
}

#[cfg(test)]
#[path = "projected_tests.rs"]
mod tests;
