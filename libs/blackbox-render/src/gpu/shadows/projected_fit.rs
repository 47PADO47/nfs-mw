//! One perspective-warped map fitted to supplied view-space boundary points.
//! Parameters and camera conventions belong to the caller; see the behavioral spec.
use blackbox_gfx::ShadowProjection;
use glam::{Mat4, Vec2, Vec3, Vec4};

#[derive(Debug, Clone, Copy)]
pub(in crate::gpu) struct Fit {
    pub clip_from_world: Mat4,
    pub near: f32,
    pub far: f32,
    pub extent: Vec2,
}

/// Convert world points and a world-space light into the explicitly supplied
/// source-view convention. This function does not assume a scene-camera handedness.
#[cfg(test)]
pub(in crate::gpu) fn fit_world_points(
    view: Mat4,
    to_light: Vec3,
    points: &[Vec3],
    parameters: &ShadowProjection,
) -> Option<Fit> {
    let view_points: Vec<_> = points.iter().map(|&p| view.transform_point3(p)).collect();
    fit_view_points(view, view.transform_vector3(to_light), &view_points, parameters)
}

/// Points and sun direction already use the original-style source-view coordinates.
/// The returned transform includes world-to-view and uses zero-to-one depth.
pub(in crate::gpu) fn fit_view_points(
    view: Mat4,
    to_light: Vec3,
    points: &[Vec3],
    parameters: &ShadowProjection,
) -> Option<Fit> {
    if points.is_empty() || !view.is_finite() || view.determinant() == 0.0 {
        return None;
    }
    let post = Mat4::from_cols_array_2d(&parameters.post_transform);
    let offset = Vec2::from_array(parameters.offset);
    let dimensions = Vec3::new(parameters.width_scale, parameters.height_scale, parameters.far_scale);
    if !post.is_finite()
        || post.determinant() == 0.0
        || !offset.is_finite()
        || !dimensions.is_finite()
        || dimensions.min_element() <= 0.0
    {
        return None;
    }
    let near = parameters.near;
    if !near.is_finite() || near <= 0.0 {
        return None;
    }
    let d = to_light.try_normalize()?;
    let r = match d.cross(Vec3::NEG_Z).try_normalize() {
        Some(r) => r,
        None => d.cross(Vec3::Y).try_normalize()?,
    };
    let u = d.cross(r).try_normalize()?;
    let basis = Mat4::from_cols(r.extend(0.0), d.extend(0.0), u.extend(0.0), Vec4::W).transpose();
    let mut lo = Vec3::splat(f32::INFINITY);
    let mut hi = Vec3::splat(f32::NEG_INFINITY);
    for &point in points {
        let point = basis.transform_point3(point);
        if !point.is_finite() {
            return None;
        }
        lo = lo.min(point);
        hi = hi.max(point);
    }
    let eye = Vec3::new((lo.x + hi.x) * 0.5 + offset.x, (lo.y + hi.y) * 0.5 + offset.y, lo.z - near);
    let mut slope = Vec2::ZERO;
    let mut far = 0.0_f32;
    for &point in points {
        let p = basis.transform_point3(point) - eye;
        if p.z <= 0.0 {
            continue;
        }
        slope = slope.max((p.truncate() / p.z).abs());
        far = far.max(p.z);
    }
    far *= parameters.far_scale;
    let extent = 2.0 * near * slope * dimensions.truncate();
    if !extent.is_finite() || extent.min_element() <= 0.0 || !far.is_finite() || far <= near {
        return None;
    }
    let depth_scale = far / (far - near);
    let perspective = Mat4::from_cols(
        Vec4::new(2.0 * near / extent.x, 0.0, 0.0, 0.0),
        Vec4::new(0.0, 2.0 * near / extent.y, 0.0, 0.0),
        Vec4::new(0.0, 0.0, depth_scale, 1.0),
        Vec4::new(0.0, 0.0, -near * depth_scale, 0.0),
    );
    let clip_from_world = post * perspective * Mat4::from_translation(-eye) * basis * view;
    if !clip_from_world.is_finite() {
        return None;
    }
    Some(Fit { clip_from_world, near, far, extent })
}

/// Original homogeneous clip-to-texture mapping with a size-dependent half texel.
pub(in crate::gpu) fn texture_bias(resolution: u32) -> Option<Mat4> {
    if resolution == 0 {
        return None;
    }
    let center = 0.5 + 0.5 / resolution as f32;
    Some(Mat4::from_cols(
        Vec4::new(0.5, 0.0, 0.0, 0.0),
        Vec4::new(0.0, -0.5, 0.0, 0.0),
        Vec4::Z,
        Vec4::new(center, center, 0.0, 1.0),
    ))
}

#[cfg(test)]
#[path = "projected_fit_tests.rs"]
pub(super) mod tests;
