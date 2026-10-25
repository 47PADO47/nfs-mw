//! Native deterministic vehicle/terrain camera noise, separate from custom shake. The player's shake multiplier
//! (`[custom_hud] camera_shake`) scales it too, so 0 is a steady camera.

use glam::{Mat4, Vec3};

use super::follow::Sample;

#[derive(Debug, Clone, Copy)]
pub struct Input<'a> {
    pub seconds: f32,
    pub speed: f32,
    pub forward_acceleration: f32,
    pub terrain: [f32; 2],
    pub nos: bool,
    pub brake: bool,
    pub drag: bool,
    pub overrev: bool,
    /// Horizontal distance to each active police helicopter.
    pub chopper_distances: &'a [f32],
    /// The player's multiplier of every amplitude, 0 to 1 (`[custom_hud] camera_shake`); 1 is the native noise.
    pub intensity: f32,
}

#[derive(Debug, Clone, Default)]
pub struct Noise {
    terrain_frequency: [f32; 4],
    terrain_amplitude: [f32; 4],
}

impl Noise {
    pub fn apply(&mut self, mode: usize, mut sample: Sample, input: Input<'_>) -> Sample {
        let intensity = crate::viewer::camera::chase::shake::scaled_intensity(input.intensity);
        let scaled = |amplitudes: [f32; 4]| amplitudes.map(|value| value * intensity);
        let chopper_scale = 1.0 - (input.speed / 30.0).clamp(0.0, 1.0) * 0.8;
        for &distance in input.chopper_distances {
            if distance >= 40.0 {
                continue;
            }
            let amplitude = chopper_scale * (1.0 - distance / 40.0);
            sample = transform(
                sample,
                input.seconds,
                // Authored 3.141f32 frequency, deliberately not mathematical PI.
                [f32::from_bits(0x4049_0625), 2.971, 0.84234, 0.92345],
                scaled([0.01, 0.05, 1.1, 2.7].map(|value| value * amplitude)),
                self.terrain_frequency,
                scaled(self.terrain_amplitude),
            );
        }
        let mut amplitude = response(input.speed, [0.0, 0.005, 0.03, 0.07, 0.02]);
        let mut frequency = response(input.speed, [0.6, 1.1, 1.2, 1.3, 1.0]);
        if input.drag {
            amplitude *= 1.5;
        }
        if input.speed > 5.0 {
            let acceleration = input.forward_acceleration.clamp(0.0, 20.0) * 0.05;
            amplitude += 0.15 * acceleration;
            frequency += (0.5 - frequency) * acceleration;
        }
        if input.overrev {
            amplitude += 0.15;
            frequency *= 0.7;
        }
        if input.nos && input.speed > 5.0 {
            amplitude = 0.3;
            frequency = 2.0;
        }
        if input.brake && input.speed > 5.0 {
            amplitude = 0.3;
            frequency = 5.0;
        }
        let mut terrain_amplitude = response(input.speed, [0.0, 0.5, 0.8, 1.0, 1.0]) * input.terrain[0];
        let terrain_frequency = response(input.speed, [1.0, 1.1, 1.2, 1.3, 1.4]) * input.terrain[1];
        if mode < 2 {
            amplitude *= 0.25;
            terrain_amplitude *= 0.25;
        }
        self.terrain_frequency = [3.0, 5.0, 7.0, 5.5].map(|value| value * terrain_frequency);
        self.terrain_amplitude = [0.007, 0.01, 0.3, 0.4].map(|value| value * terrain_amplitude);
        transform(
            sample,
            input.seconds,
            [1.8, 2.0, 2.125, 2.0].map(|value| value * frequency),
            scaled([0.03, 0.025, 0.68, 0.28].map(|value| value * amplitude)),
            self.terrain_frequency,
            scaled(self.terrain_amplitude),
        )
    }
}

fn response(speed: f32, values: [f32; 5]) -> f32 {
    let fraction = (speed * 0.05).clamp(0.0, 4.0);
    let at = (fraction as usize).min(3);
    let t = fraction - at as f32;
    values[at] * (1.0 - t) + values[at + 1] * t
}

fn transform(
    sample: Sample,
    seconds: f32,
    frequency: [f32; 4],
    amplitude: [f32; 4],
    frequency2: [f32; 4],
    amplitude2: [f32; 4],
) -> Sample {
    let channels: [f32; 4] = std::array::from_fn(|i| {
        wave(frequency[i] * seconds) * amplitude[i] + wave(frequency2[i] * seconds) * amplitude2[i]
    });
    if channels == [0.0; 4] {
        return sample;
    }
    let pitch = relative_angle(channels[2], sample.fov_bangle);
    let yaw = relative_angle(channels[3], sample.fov_bangle);
    let correction = Mat4::from_rotation_y(-yaw)
        * Mat4::from_rotation_x(pitch)
        * Mat4::from_translation(Vec3::new(channels[0], -channels[1], 0.0));
    let view = glam::camera::rh::view::look_at_mat4(sample.eye, sample.target, sample.up);
    let inverse = (correction * view).inverse();
    let eye = inverse.w_axis.truncate();
    let distance = sample.eye.distance(sample.target);
    Sample {
        eye,
        target: eye - inverse.z_axis.truncate().normalize_or(Vec3::Z) * distance,
        up: inverse.y_axis.truncate().normalize_or(Vec3::Y),
        ..sample
    }
}

fn relative_angle(degrees: f32, fov: f32) -> f32 {
    let radians_per_unit = std::f32::consts::TAU / 65536.0;
    let angle = ((degrees * 65536.0) as i32 / 360) as u16;
    let fov_half = (fov as u16) >> 1;
    let sine = (f32::from(angle) * radians_per_unit).sin() * (f32::from(fov_half) * radians_per_unit).sin()
        / (5461.0 * radians_per_unit).sin();
    let result = sine.clamp(-1.0, 1.0).asin();
    ((result / radians_per_unit) as i32) as f32 * radians_per_unit
}

fn wave(phase: f32) -> f32 {
    let mut result = 0.0_f32;
    let mut frequency = 1.0;
    let mut amplitude = 1.0;
    for _ in 0..6 {
        result = (interpolated(phase * frequency) * amplitude + f64::from(result)) as f32;
        frequency *= 2.0;
        amplitude *= 0.5;
    }
    result
}

fn interpolated(phase: f32) -> f64 {
    let integer = phase.floor() as i32;
    let fraction = f64::from(phase) - f64::from(integer);
    lattice(integer.wrapping_add(1)) * fraction + (1.0 - fraction) * lattice(integer)
}

fn lattice(integer: i32) -> f64 {
    let integer = integer as u32;
    let value = (integer << 13) ^ integer;
    let hash =
        value.wrapping_mul(value).wrapping_mul(15731).wrapping_add(789221).wrapping_mul(value).wrapping_add(1376312589)
            & 0x7fff_ffff;
    1.0 - f64::from(hash) / 1073741824.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn six_octaves_match_unchanged_native_waveform_vectors() {
        for (input, expected) in [
            (-17.125, -0.62633973),
            (-1.5, -0.11356238),
            (-0.125, 0.03719838),
            (0.0, -0.554776),
            (0.01, -0.5514509),
            (0.125, -0.46399206),
            (0.5, -0.25505826),
            (1.0, 0.006913488),
            (2.25, 0.17706431),
            (17.125, -0.083756305),
            (123.456, -0.07443597),
        ] {
            assert!((wave(input) - expected).abs() < 1e-7, "phase {input}");
        }
    }

    #[test]
    fn rotations_match_native_fov_relative_noise_vectors() {
        let unit = std::f32::consts::TAU / 65536.0;
        for (degrees, fov, expected) in [
            (-0.16310415, 14198.0, -36.0),
            (-0.113174304, 14198.0, -25.0),
            (-0.09050327, 14198.0, -20.0),
            (-0.06864071, 14198.0, -15.0),
            (-0.039219745, 14198.0, -8.0),
            (0.06260578, 14198.0, 13.0),
            (-0.039219745, 9830.0, -6.0),
            (0.06260578, 9830.0, 9.0),
            (0.78853124, 16384.0, 202.0),
            (-0.12287369, 16384.0, -31.0),
        ] {
            assert!((relative_angle(degrees, fov) / unit - expected).abs() < 0.01);
        }
    }

    fn shaking_input(intensity: f32) -> Input<'static> {
        Input {
            seconds: 0.37,
            speed: 30.0,
            forward_acceleration: 8.0,
            terrain: [1.0, 1.0],
            nos: false,
            brake: true,
            drag: false,
            overrev: false,
            chopper_distances: &[10.0],
            intensity,
        }
    }

    #[test]
    fn the_shake_intensity_scales_the_noise_and_zero_holds_the_camera_steady() {
        let sample = Sample { eye: Vec3::ZERO, target: -Vec3::Z, up: Vec3::Y, fov_bangle: 14198.0 };
        let moved = |intensity: f32| {
            let mut noise = Noise::default();
            // A second frame also runs the helicopter pass on the stored terrain noise.
            noise.apply(3, sample, shaking_input(intensity));
            let result = noise.apply(3, sample, shaking_input(intensity));
            result.eye.distance(sample.eye) + result.target.distance(sample.target)
        };
        let full = moved(1.0);
        assert!(full > 1e-4, "braking at speed over rough ground shakes the camera");
        assert_eq!(moved(f32::NAN), full, "a value that is not a number keeps the native noise");
        let half = moved(0.5);
        assert!(half > 0.0 && half < full, "half is between steady and native: {half} of {full}");
        let mut noise = Noise::default();
        noise.apply(3, sample, shaking_input(0.0));
        assert_eq!(noise.apply(3, sample, shaking_input(0.0)), sample, "0 is a steady camera");
    }

    #[test]
    fn screen_translation_and_rotation_match_native_matrix_order() {
        let sample = Sample { eye: Vec3::ZERO, target: -Vec3::Z, up: Vec3::Y, fov_bangle: 14198.0 };
        let result = transform(
            sample,
            0.0,
            [1.8, 2.0, 2.125, 2.0],
            [0.03, 0.025, 0.68, 0.28].map(|a| a * 0.3),
            [3.0, 5.0, 7.0, 5.5],
            [0.007, 0.01, 0.3, 0.4].map(|a| a * 0.3),
        );
        let native = Mat4::from_cols_array(&[
            1.0000006,
            0.0,
            0.0023968427,
            0.0,
            0.000008272583,
            0.9999975,
            -0.0034514521,
            0.0,
            -0.0023968366,
            0.00345145,
            0.9999981,
            0.0,
            -0.006158065,
            -0.0058251335,
            0.000005345429,
            1.0,
        ]);
        let flip = Mat4::from_scale(Vec3::new(1.0, -1.0, -1.0));
        let expected = flip * native * flip;
        let actual = glam::camera::rh::view::look_at_mat4(result.eye, result.target, result.up);
        for (actual, expected) in actual.to_cols_array().into_iter().zip(expected.to_cols_array()) {
            assert!((actual - expected).abs() < 1e-5, "{actual} != {expected}");
        }
    }
}
