//! Stable enclosing spheres and texel snapping; no previous-frame transforms or image history.

use blackbox_gfx::{FrameParams, Projection, RenderExperiments};
use glam::{Mat4, Vec3};

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub(super) struct Params {
    pub matrices: [[f32; 16]; 4],
    pub splits: [f32; 4],
    pub options: [f32; 4],
    pub camera: [f32; 4],
    pub forward: [f32; 4],
    pub sun: [f32; 4],
    pub texels: [f32; 4],
    pub depth_spans: [f32; 4],
    pub softness: [f32; 4],
    pub contact: [f32; 4],
    pub ambient: [f32; 4],
    pub environment: [f32; 4],
    pub world: [f32; 4],
    pub debug: [f32; 4],
}

pub(super) struct Fit {
    pub params: Params,
    pub count: usize,
}

pub(super) fn fit(frame: &FrameParams, settings: RenderExperiments) -> Option<Fit> {
    let Projection::PerspectiveInfiniteReverse { fov_y, aspect, near } = frame.projection else { return None };
    let settings = settings.sanitized();
    let inverse = frame.view.inverse();
    let forward = inverse.transform_vector3(Vec3::NEG_Z).normalize_or_zero();
    let sun = (-frame.light_dir).try_normalize().unwrap_or(Vec3::Z);
    let up = match sun.z.abs() > 0.95 {
        true => Vec3::Y,
        false => Vec3::Z,
    };
    let light_rotation = glam::camera::rh::view::look_at_mat4(Vec3::ZERO, -sun, up);
    let distance = settings.shadow_distance;
    let splits = [distance * 0.08, distance * 0.28, distance, settings.caster_distance()];
    let count = match settings.far_shadows {
        true => 4,
        false => 3,
    };
    let mut matrices = [Mat4::IDENTITY.to_cols_array(); 4];
    let tangent = (fov_y * 0.5).tan();
    let mut texels = [0.0; 4];
    let mut depth_spans = [0.0; 4];
    for i in 0..count {
        let start = match i {
            0 => near,
            _ => splits[i - 1] * 0.8,
        };
        let end = splits[i];
        // Minimum enclosing sphere of the symmetric near/far rectangles, independent of orientation.
        let k = tangent.powi(2) * (1.0 + aspect * aspect);
        let center_z = ((start + end) * (1.0 + k) * 0.5).min(end);
        let center_view = Vec3::new(0.0, 0.0, -center_z);
        let radius = (end * end * k + (end - center_z).powi(2)).sqrt();
        let radius = (radius * 16.0).ceil() / 16.0;
        // Include half-texel snapping and the largest blocker/filter footprint at the map boundary.
        let size = settings.shadow_resolution as f32;
        let radius = radius * size / (size - 32.0);
        let center = inverse.transform_point3(center_view);
        let texel = 2.0 * radius / settings.shadow_resolution as f32;
        let light_center = light_rotation.transform_point3(center);
        let snapped = Vec3::new(
            (light_center.x / texel).round() * texel,
            (light_center.y / texel).round() * texel,
            (light_center.z / texel).round() * texel,
        );
        let snapped_world = light_rotation.inverse().transform_point3(snapped);
        // Extra upstream depth includes tall and off-camera casters without changing XY fit.
        let margin = radius + 180.0;
        texels[i] = texel;
        depth_spans[i] = 2.0 * margin - 0.1;
        let view = glam::camera::rh::view::look_at_mat4(snapped_world + sun * margin, snapped_world, up);
        let projection =
            glam::camera::rh::proj::directx::orthographic(-radius, radius, -radius, radius, 0.1, 2.0 * margin);
        matrices[i] = (projection * view).to_cols_array();
    }
    Some(Fit {
        params: Params {
            matrices,
            splits,
            options: [
                u8::from(settings.sun_shadows) as f32,
                settings.shadow_resolution as f32,
                settings.shadow_filter,
                u8::from(settings.clearcoat) as f32,
            ],
            camera: frame.camera_position.extend(count as f32).to_array(),
            forward: forward.extend(0.0).to_array(),
            sun: sun.extend(0.0).to_array(),
            texels,
            depth_spans,
            softness: [settings.sun_angle.to_radians().tan(), u8::from(settings.soft_shadows) as f32, 0.0, 0.0],
            contact: [0.0; 4],
            ambient: [0.0; 4],
            environment: [0.0; 4],
            world: [0.0; 4],
            debug: [0.0; 4],
        },
        count,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn snapping_stabilizes_lateral_subtexel_movement() {
        let mut frame = FrameParams {
            view: glam::camera::rh::view::look_at_mat4(Vec3::new(0.0, -10.0, 3.0), Vec3::ZERO, Vec3::Z),
            projection: Projection::PerspectiveInfiniteReverse { fov_y: 1.0, aspect: 1.5, near: 0.1 },
            camera_position: Vec3::new(0.0, -10.0, 3.0),
            light_dir: Vec3::NEG_Z,
            clear_color: [0.0; 3],
            fog: None,
            scattering: None,
            surface_light: None,
            camera_cut: false,
            shadow_tuning: None,
        };
        let before = fit(&frame, RenderExperiments::default()).unwrap();
        frame.view *= Mat4::from_translation(Vec3::new(-0.00001, 0.0, 0.0));
        let after = fit(&frame, RenderExperiments::default()).unwrap();
        assert_eq!(before.params.matrices, after.params.matrices);
        let far =
            fit(&frame, RenderExperiments { far_shadows: true, sun_shadows: true, ..Default::default() }).unwrap();
        assert_eq!(far.count, 4);
        assert_eq!(far.params.splits[3], 480.0);
    }

    #[test]
    fn camera_frusta_remain_inside_the_filter_guard_band() {
        let eye = Vec3::new(3.0, -10.0, 5.0);
        for aspect in [1.5, 2.0] {
            let frame = FrameParams {
                view: glam::camera::rh::view::look_at_mat4(eye, Vec3::ZERO, Vec3::Z),
                projection: Projection::PerspectiveInfiniteReverse { fov_y: 1.0, aspect, near: 0.1 },
                camera_position: eye,
                light_dir: Vec3::new(0.3, -0.2, -1.0),
                clear_color: [0.0; 3],
                fog: None,
                scattering: None,
                surface_light: None,
                camera_cut: false,
                shadow_tuning: None,
            };
            for resolution in [512, 4096] {
                let fit = fit(
                    &frame,
                    RenderExperiments {
                        sun_shadows: true,
                        far_shadows: true,
                        shadow_resolution: resolution,
                        ..Default::default()
                    },
                )
                .unwrap();
                for i in 0..fit.count {
                    let matrix = Mat4::from_cols_array(&fit.params.matrices[i]);
                    let start = match i {
                        0 => 0.1,
                        _ => fit.params.splits[i - 1] * 0.8,
                    };
                    for z in [start, fit.params.splits[i]] {
                        for x in [-1.0, 1.0] {
                            for y in [-1.0, 1.0] {
                                let world = frame.view.inverse().transform_point3(Vec3::new(
                                    x * z * 0.5_f32.tan() * aspect,
                                    y * z * 0.5_f32.tan(),
                                    -z,
                                ));
                                let p = matrix.transform_point3(world);
                                let limit = 1.0 - 24.0 / resolution as f32;
                                assert!(p.x.abs() <= limit && p.y.abs() <= limit, "cascade {i} filter coverage: {p:?}");
                                assert!((0.0..=1.0).contains(&p.z));
                            }
                        }
                    }
                }
            }
        }
    }
}
