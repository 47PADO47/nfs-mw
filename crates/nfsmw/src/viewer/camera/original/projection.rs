//! Native angle finalization and the playable reference's Scaling0 optical axes.

use super::follow::Sample;
use crate::viewer::camera::Lens;

const UNITS_PER_TURN: f32 = 65536.0;
const NEAR: f32 = 0.5;
#[cfg(test)]
const MAIN_VIEW_VERTICAL_SCALE: f32 = 1.15;

/// Finalize the cubic FOV before collision routines consume the camera angle.
pub(super) fn finalize_sample(mut sample: Sample) -> Sample {
    sample.fov_bangle = f32::from(final_angle(sample.fov_bangle).max(2));
    sample
}

/// Build the lens from a finalized sample, without repeating the cubic angle round trip.
pub(super) fn lens_finalized(sample: Sample, aspect: f32) -> Lens {
    let full = sample.fov_bangle.clamp(2.0, 32766.0) as u16;
    let horizontal = full >> 1;
    let vertical = (f64::from(full) * f64::from(1.15f32) * f64::from(0.43434f32)).clamp(1.0, 16383.0) as u16;
    let radians = std::f32::consts::TAU / UNITS_PER_TURN;
    let x = ((4.0 / 3.0) / aspect) / (f32::from(horizontal) * radians).tan();
    let y = 1.215 / (f32::from(vertical) * radians).tan();
    lens_scales(sample, x, y)
}

#[cfg(test)]
fn stock_lens(sample: Sample, aspect: f32) -> Lens {
    let (horizontal, vertical) = half_angles(sample.fov_bangle.clamp(2.0, 32766.0) as u16, aspect);
    lens_scales(sample, horizontal.tan().recip(), vertical.tan().recip())
}

fn lens_scales(sample: Sample, x: f32, y: f32) -> Lens {
    let effective_aspect = y / x;
    let fov_y_degrees = (2.0 * y.recip().atan()).to_degrees();
    let mut lens = Lens::look_at(sample.eye, sample.target, fov_y_degrees, NEAR, effective_aspect);
    lens.view = glam::camera::rh::view::look_at_mat4(sample.eye, sample.target, sample.up);
    lens
}

#[cfg(test)]
fn lens(sample: Sample, aspect: f32) -> Lens {
    lens_finalized(finalize_sample(sample), aspect)
}

#[cfg(test)]
fn half_angles(full: u16, aspect: f32) -> (f32, f32) {
    // Native camera output is a 16-bit whole angle, followed by two independent
    // truncating half-angle conversions. Preserve the pair of quantization steps.
    let horizontal = (f32::from(full) * 0.5) as u16;
    let ratio = aspect.recip() * MAIN_VIEW_VERTICAL_SCALE;
    let vertical = (f32::from(full) * 0.5 * ratio).clamp(1.0, 16383.0) as u16;
    let radians_per_unit = std::f32::consts::TAU / UNITS_PER_TURN;
    (f32::from(horizontal) * radians_per_unit, f32::from(vertical) * radians_per_unit)
}

fn final_angle(current: f32) -> u16 {
    let half = (current.clamp(2.0, 32766.0) as u16) >> 1;
    let tangent = (f64::from(sine(half) as f32) / sine(half + 16384)) as f32;
    let ratio = f64::from(tangent.min(1.0)) / f64::from(tangent.max(1.0));
    let fixed = (ratio * 65536.0) as u32;
    let index = fixed >> 8;
    let endpoint = |i: u32| ((f64::from(i) / 256.0).atan() * 65536.0 / std::f64::consts::TAU).round() as u32;
    let low = endpoint(index);
    let angle = low + (((endpoint(index + 1) - low) * (fixed & 255)) >> 8);
    if tangent > 1.0 {
        return ((16384 - angle) * 2) as u16;
    }
    (angle * 2) as u16
}

fn sine(angle: u16) -> f64 {
    let radians = f64::from(angle) * f64::from(std::f32::consts::TAU / UNITS_PER_TURN);
    let mut x = radians;
    let mut sign = 1.0;
    if x >= f64::from(std::f32::consts::FRAC_PI_2) {
        x -= f64::from(std::f32::consts::PI);
        sign = -1.0;
    }
    let square = x * x;
    let fifth = square * x * square;
    let seventh = fifth * square;
    sign * (seventh * square * f64::from(1.0f32 / 362880.0)
        + (fifth * f64::from(1.0f32 / 120.0) + (x - square * x * f64::from(1.0f32 / 6.0)))
        - seventh * f64::from(1.0f32 / 5040.0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::{Vec3, Vec4};

    fn sample(fov_bangle: f32) -> Sample {
        Sample { eye: Vec3::new(-5.0, 0.0, 2.0), target: Vec3::new(0.0, 0.0, 1.0), up: Vec3::Z, fov_bangle }
    }

    #[test]
    fn independent_axes_match_native_pc_projection_vectors() {
        // Isolated unchanged original projection with synthetic inputs; no game assets.
        let vectors = [
            (10923.0, 800.0, 600.0, 1.7321787, 2.0619092),
            (10923.0, 1920.0, 1080.0, 1.7321787, 2.8393612),
            (10923.0, 3440.0, 1440.0, 1.7321787, 3.8830583),
            (14199.0, 800.0, 600.0, 1.2350746, 1.5031484),
            (14199.0, 1920.0, 1080.0, 1.2350746, 2.1227412),
            (14199.0, 3440.0, 1440.0, 1.2350746, 2.942507),
            (16384.0, 800.0, 600.0, 1.0, 1.2433401),
            (16384.0, 1920.0, 1080.0, 1.0, 1.7960349),
            (16384.0, 3440.0, 1440.0, 1.0, 2.5180633),
        ];
        for (fov, width, height, x, y) in vectors {
            let (horizontal, vertical) = half_angles(fov as u16, width / height);
            assert!((horizontal.tan().recip() - x).abs() < 5e-5, "fov={fov}, {width}x{height}");
            assert!((vertical.tan().recip() - y).abs() < 5e-5, "fov={fov}, {width}x{height}");
        }
    }

    #[test]
    fn final_cubic_fov_round_trip_matches_native_vectors() {
        for (input, expected) in [
            (4000.0, 4000),
            (5461.0, 5458),
            (10922.0, 10920),
            (10923.0, 10920),
            (14198.0, 14196),
            (14199.0, 14196),
            (16384.0, 16382),
            (16385.0, 16382),
            (20001.0, 20000),
            (14199.9, 14196),
        ] {
            assert_eq!(final_angle(input), expected, "{input}");
        }
    }

    #[test]
    fn final_cubic_fov_and_projection_match_the_composed_native_path() {
        let vectors = [
            (10923.0, 800.0, 600.0, 1.7325623, 2.0624127),
            (10923.0, 1920.0, 1080.0, 1.7325623, 2.8402302),
            (10923.0, 3440.0, 1440.0, 1.7325623, 3.8846004),
            (14199.0, 800.0, 600.0, 1.2353169, 1.503461),
            (14199.0, 1920.0, 1080.0, 1.2353169, 2.1232693),
            (14199.0, 3440.0, 1440.0, 1.2353169, 2.9434333),
            (16384.0, 800.0, 600.0, 1.0001918, 1.2435842),
            (16384.0, 1920.0, 1080.0, 1.0001918, 1.7964401),
            (16384.0, 3440.0, 1440.0, 1.0001918, 2.5180633),
        ];
        for (fov, width, height, x, y) in vectors {
            let matrix = stock_lens(finalize_sample(sample(fov)), width / height).projection.matrix();
            assert!((matrix.x_axis.x - x).abs() < 5e-5, "fov={fov}, {width}x{height}");
            assert!((matrix.y_axis.y - y).abs() < 5e-5, "fov={fov}, {width}x{height}");
        }
    }

    #[test]
    fn playable_reference_projection_matches_composed_scaling_zero_vectors() {
        // Source-described official hooks composed with native cubic finalization.
        let vectors = [
            (10923.0, 800.0, 600.0, 1.7325623, 2.107863),
            (10923.0, 1920.0, 1080.0, 1.2994218, 2.107863),
            (10923.0, 3440.0, 1440.0, 0.9670116, 2.107863),
            (14199.0, 800.0, 600.0, 1.2353169, 1.5032662),
            (14199.0, 1920.0, 1080.0, 0.92648774, 1.5032662),
            (14199.0, 3440.0, 1440.0, 0.68947923, 1.5032662),
            (16384.0, 800.0, 600.0, 1.0001918, 1.217332),
            (16384.0, 1920.0, 1080.0, 0.7501439, 1.217332),
            (16384.0, 3440.0, 1440.0, 0.5582466, 1.217332),
        ];
        for (fov, width, height, x, y) in vectors {
            let matrix = lens(sample(fov), width / height).projection.matrix();
            assert!((matrix.x_axis.x - x).abs() < 5e-5, "fov={fov}, {width}x{height}");
            assert!((matrix.y_axis.y - y).abs() < 5e-5, "fov={fov}, {width}x{height}");
            let point = matrix * Vec4::new(2.0, 3.0, -10.0, 1.0);
            assert!((point.x / point.w - x * 0.2).abs() < 1e-5);
            assert!((point.y / point.w - y * 0.3).abs() < 2e-5);
        }
    }

    #[test]
    fn native_near_plane_and_tilt_survive_the_renderer_adapter() {
        let mut pose = sample(16384.0);
        pose.up = Vec3::new(0.0, 0.4, 1.0).normalize();
        let lens = lens(pose, 16.0 / 9.0);
        assert_eq!(lens.position, pose.eye);
        let orthogonal_up = (pose.up - lens.forward * pose.up.dot(lens.forward)).normalize();
        assert!((lens.view.inverse().y_axis.truncate().normalize() - orthogonal_up).length() < 1e-5);
        let clip = lens.projection.matrix() * Vec4::new(0.0, 0.0, -NEAR, 1.0);
        assert!((clip.z / clip.w - 1.0).abs() < 1e-5);
        assert!(lens.view_proj().is_finite());
    }
}
