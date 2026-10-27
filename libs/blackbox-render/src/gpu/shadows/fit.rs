//! Stable enclosing spheres and local-pivot texel snapping; no image history.

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
    pub projected: [f32; 4],
    pub resolutions: [f32; 4],
    pub cloud: [f32; 4],
    pub world_light: [[f32; 4]; 9],
}

/// Snap step per cascade in texels. Coarser steps move the map origin by at most half a step, which
/// stays inside the fit's guard band.
const SNAP_TEXELS: [f32; 4] = [1.0, 2.0, 4.0, 4.0];

pub(super) struct Fit {
    pub params: Params,
    pub count: usize,
}

#[cfg(test)]
fn fit(frame: &FrameParams, settings: RenderExperiments, casters: &[(usize, Vec3, f32)]) -> Option<Fit> {
    fit_with_grid(frame, settings, casters, &mut super::grid::Grid::default())
}

pub(super) fn fit_with_grid(
    frame: &FrameParams,
    settings: RenderExperiments,
    casters: &[(usize, Vec3, f32)],
    grid: &mut super::grid::Grid,
) -> Option<Fit> {
    let Projection::PerspectiveInfiniteReverse { fov_y, aspect, near } = frame.projection else { return None };
    let settings = settings.sanitized();
    let inverse = frame.view.inverse();
    let forward = inverse.transform_vector3(Vec3::NEG_Z).normalize_or_zero();
    let sun = (-frame.light_dir).try_normalize().unwrap_or(Vec3::Z);
    // Keep one continuous daylight basis through the old .95 zenith switch.
    // This shortest rotation of the world XY axes is singular only at the nadir.
    let up = match sun.z < -0.9999 {
        true => Vec3::NEG_Y,
        false => Vec3::new(-sun.x * sun.y / (1.0 + sun.z), 1.0 - sun.y * sun.y / (1.0 + sun.z), -sun.y),
    };
    let light_rotation = glam::camera::rh::view::look_at_mat4(Vec3::ZERO, -sun, up);
    let rebase = grid.follow(frame.camera_position);
    // One light-space transform per caster per frame; every cascade's depth fit reuses it.
    let mut light_bounds = std::mem::take(&mut grid.light_bounds);
    light_bounds.clear();
    for &(_, position, radius) in casters {
        light_bounds.push((light_rotation.transform_point3(position), radius));
    }
    let distance = settings.shadow_distance;
    let near_share = match settings.near_shadow_focus {
        true => 0.04,
        false => 0.08,
    };
    let mut splits = [distance * near_share, distance * 0.28, distance, settings.caster_distance()];
    let mut resolutions = [settings.shadow_resolution as f32; 4];
    let mut overlap = 0.8;
    if settings.wide_shadow_detail {
        splits = [
            distance * near_share * 2.0,
            distance * if settings.near_shadow_focus { 0.32 } else { 0.42 },
            distance,
            settings.caster_distance(),
        ];
        resolutions[..2].fill(settings.shadow_detail_resolution as f32);
        overlap = 0.65;
    }
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
            _ => splits[i - 1] * overlap,
        };
        let end = splits[i];
        // Minimum enclosing sphere of the symmetric near/far rectangles, independent of orientation.
        let k = tangent.powi(2) * (1.0 + aspect * aspect);
        let center_z = ((start + end) * (1.0 + k) * 0.5).min(end);
        let center_view = Vec3::new(0.0, 0.0, -center_z);
        let radius = (end * end * k + (end - center_z).powi(2)).sqrt();
        let radius = (radius * 16.0).ceil() / 16.0;
        // Include half-texel snapping and the largest blocker/filter footprint at the map boundary.
        let size = resolutions[i];
        let radius = radius * size / (size - 32.0);
        let center = inverse.transform_point3(center_view);
        let texel = 2.0 * radius / size;
        // Far cascades snap to coarser steps (still whole texels), so their matrices stay bit-identical
        // across many frames of camera motion and the static-depth cache can hit.
        let snapped = grid.snap(light_rotation, center, texel * SNAP_TEXELS[i], i, rebase);
        // Preserve stable XY density; expand only depth for upstream resident casters.
        let margin = radius + 180.0;
        let (bottom, top) = depth_bounds(snapped, radius, margin, &light_bounds);
        let depth = top - bottom;
        texels[i] = texel;
        depth_spans[i] = depth;
        // Construct from the fixed rotation directly: subtracting eye - sun at
        // city-sized coordinates would lose direction precision and rotate the grid.
        let view = Mat4::from_translation(Vec3::new(-snapped.x, -snapped.y, -top - 0.1)) * light_rotation;
        let projection =
            glam::camera::rh::proj::directx::orthographic(-radius, radius, -radius, radius, 0.1, depth + 0.1);
        matrices[i] = (projection * view).to_cols_array();
    }
    grid.light_bounds = light_bounds;
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
            softness: [
                settings.sun_angle.to_radians().tan(),
                u8::from(settings.soft_shadows) as f32,
                u8::from(settings.dense_shadow_filter) as f32,
                u8::from(settings.wide_shadow_detail) as f32,
            ],
            contact: [0.0; 4],
            ambient: [0.0; 4],
            environment: [0.0; 4],
            world: [0.0; 4],
            debug: [0.0; 4],
            projected: [0.0; 4],
            resolutions,
            cloud: [0.0; 4],
            world_light: [[0.0; 4]; 9],
        },
        count,
    })
}

fn depth_bounds(center: Vec3, radius: f32, margin: f32, casters: &[(Vec3, f32)]) -> (f32, f32) {
    let mut top = center.z + margin;
    for &(p, caster_radius) in casters {
        if (p.x - center.x).abs() > radius + caster_radius || (p.y - center.y).abs() > radius + caster_radius {
            continue;
        }
        top = top.max(p.z + caster_radius + 1.0);
    }
    // Quantise world depth, independently of frame time and candidate iteration order.
    (((center.z - margin) / 16.0).floor() * 16.0, (top / 16.0).ceil() * 16.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn frame(sun: Vec3) -> FrameParams {
        let eye = Vec3::new(3.0, -8.0, 4.0);
        FrameParams {
            view: glam::camera::rh::view::look_at_mat4(eye, Vec3::ZERO, Vec3::Z),
            projection: Projection::PerspectiveInfiniteReverse { fov_y: 1.0, aspect: 1.5, near: 0.1 },
            camera_position: eye,
            light_dir: -sun,
            clear_color: [0.0; 3],
            fog: None,
            scattering: None,
            surface_light: None,
            camera_cut: false,

            shadow_tuning: None,
        }
    }

    #[test]
    fn upstream_depth_expansion_preserves_xy_resolution_and_ignores_unrelated_casters() {
        let sun = Vec3::new(0.0, -0.98, 0.2).normalize();
        let frame = frame(sun);
        let settings = RenderExperiments::default();
        let before = fit(&frame, settings, &[]).unwrap();
        let casters = [(0, sun * 240.0, 3.0), (1, Vec3::splat(10000.0), 1.0)];
        let after = fit(&frame, settings, &casters).unwrap();
        let isolated = fit(&frame, settings, &casters[..1]).unwrap();
        assert_eq!(after.params.matrices, isolated.params.matrices, "unrelated scenery cannot inflate depth");
        assert_eq!(before.params.texels, after.params.texels, "extra depth must not dilute XY density");
        let a = Mat4::from_cols_array(&before.params.matrices[0]);
        let b = Mat4::from_cols_array(&after.params.matrices[0]);
        assert!(a.transform_point3(casters[0].1).z < 0.0, "old slab excludes upstream caster");
        assert!((0.0..=1.0).contains(&b.transform_point3(casters[0].1).z));
        for point in [Vec3::ZERO, Vec3::X * 5.0, Vec3::Y * 5.0] {
            assert!((a.transform_point3(point).truncate() - b.transform_point3(point).truncate()).length() < 1e-5);
        }
        assert_eq!(after.params.matrices, fit(&frame, settings, &[casters[1], casters[0]]).unwrap().params.matrices);
    }

    #[test]
    fn daylight_basis_is_continuous_through_the_former_zenith_switch() {
        let settings = RenderExperiments::default();
        let coordinates = [0.94999_f32, 0.95001].map(|z| {
            let fit = fit(&frame(Vec3::new((1.0 - z * z).sqrt(), 0.0, z)), settings, &[]).unwrap();
            Mat4::from_cols_array(&fit.params.matrices[0]).transform_point3(Vec3::new(1.0, 2.0, 0.0))
        });
        assert!(
            (coordinates[0].truncate() - coordinates[1].truncate()).length() < 0.005,
            "sun motion must not rotate the texel grid by ninety degrees"
        );
    }

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
        let before = fit(&frame, RenderExperiments::default(), &[]).unwrap();
        frame.view *= Mat4::from_translation(Vec3::new(-0.00001, 0.0, 0.0));
        let after = fit(&frame, RenderExperiments::default(), &[]).unwrap();
        assert_eq!(before.params.matrices, after.params.matrices);
        let far =
            fit(&frame, RenderExperiments { far_shadows: true, sun_shadows: true, ..Default::default() }, &[]).unwrap();
        assert_eq!(far.count, 4);
        assert_eq!(far.params.splits[3], 480.0);
    }

    #[test]
    fn wide_detail_combines_tight_focus_and_retains_distant_coverage() {
        let settings = RenderExperiments {
            sun_shadows: true,
            far_shadows: true,
            shadow_distance: 320.0,
            wide_shadow_detail: true,
            near_shadow_focus: true,
            shadow_resolution: 4096,
            shadow_detail_resolution: 8192,
            ..Default::default()
        };
        let fit = fit(&frame(Vec3::new(0.3, -0.2, 1.0)), settings, &[]).unwrap();
        assert_eq!(fit.params.resolutions, [8192.0, 8192.0, 4096.0, 4096.0]);
        assert!((fit.params.splits[0] * 0.65 - 16.64).abs() < 0.001);
        assert!((fit.params.splits[1] - 102.4).abs() < 0.001);
        assert_eq!(fit.params.splits[3], 960.0);
        assert!(fit.params.texels[1] / fit.params.texels[0] < 4.1, "avoid the focused layout's sevenfold density step");
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
            for (resolution, focus, wide) in [
                (512, false, false),
                (4096, false, false),
                (4096, true, false),
                (8192, false, false),
                (8192, true, false),
                (512, false, true),
                (4096, true, true),
            ] {
                let fit = fit(
                    &frame,
                    RenderExperiments {
                        sun_shadows: true,
                        far_shadows: true,
                        shadow_resolution: resolution,
                        near_shadow_focus: focus,
                        wide_shadow_detail: wide,
                        shadow_detail_resolution: resolution * 2,
                        ..Default::default()
                    },
                    &[],
                )
                .unwrap();
                for i in 0..fit.count {
                    let matrix = Mat4::from_cols_array(&fit.params.matrices[i]);
                    let start = match i {
                        0 => 0.1,
                        _ if wide => fit.params.splits[i - 1] * 0.65,
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
                                let limit = 1.0 - 24.0 / fit.params.resolutions[i];
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
