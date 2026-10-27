//! Stable, low-order diffuse sky irradiance. No local capture, rays or frame history.
//! The analytic sky follows sky_enhanced.wgsl; weather uses its broad coverage average.
use blackbox_gfx::{FrameParams, SkyState};
use glam::Vec3;

const SAMPLES: usize = 128;

pub(super) fn coefficients(frame: &FrameParams, sky: Option<SkyState>) -> [[f32; 4]; 9] {
    let state = sky.unwrap_or_default().sanitized();
    let sun = frame.scattering.map_or(-frame.light_dir, |s| s.to_sun).normalize_or_zero();
    let coverage = finite(state.cloud_coverage, 0.0).clamp(0.0, 1.0);
    let fog = Vec3::from_array(frame.clear_color).clamp(Vec3::ZERO, Vec3::ONE);
    let ambient = frame.surface_light.map_or(Vec3::splat(0.35), |s| Vec3::from_array(s.ambient));
    let direct = frame.surface_light.map_or(Vec3::splat(0.65), |s| Vec3::from_array(s.diffuse));
    let sky_at = |ray| {
        if state.enhanced {
            return atmosphere(ray, sun, fog, coverage, state.sky_sunset_warmth);
        }
        // Authored sky has no radiance field. Use its ambient/fog colours without
        // assuming its textures are a modern, linear-light environment map.
        ambient.max(Vec3::splat(0.01)).lerp(fog.max(Vec3::splat(0.01)), 0.35)
    };
    let mean_sky = (0..32)
        .map(|i| {
            let ray = direction(i, 32);
            sky_at(Vec3::new(ray.x, ray.y, ray.z.abs()))
        })
        .sum::<Vec3>()
        / 32.0;
    // Generic neutral ground albedo, not the player's road/cubemap. Preserve the
    // authored ambient energy below; this only supplies a restrained bounce direction.
    let ground = mean_sky * Vec3::new(0.40, 0.36, 0.30)
        + direct.max(Vec3::ZERO) * sun.z.max(0.0) * Vec3::new(0.10, 0.085, 0.065);
    let mut result = [Vec3::ZERO; 9];
    let mut average = Vec3::ZERO;
    for i in 0..SAMPLES {
        let ray = direction(i, SAMPLES);
        let radiance = match ray.z >= 0.0 {
            true => sky_at(ray),
            false => ground,
        };
        // The native material contract is display encoded. Project into that same
        // domain rather than multiplying a linear radiance into encoded artwork.
        let radiance = radiance.max(Vec3::ZERO).map(|v| {
            if v <= 0.0031308 {
                return 12.92 * v;
            }
            1.055 * v.powf(1.0 / 2.4) - 0.055
        });
        average += radiance;
        for (coefficient, basis) in result.iter_mut().zip(basis(ray)) {
            *coefficient += radiance * basis;
        }
    }
    // Authored ambient already carries the weather/time colour. Normalize each
    // channel so the sky adds directional colour variation, not a second global tint.
    let reference = (average / SAMPLES as f32).max(Vec3::splat(0.01));
    std::array::from_fn(|i| {
        // Cosine convolution / PI: band 0 = 1, band 1 = 2/3, band 2 = 1/4.
        let convolution = match i {
            0 => 1.0,
            1..=3 => 2.0 / 3.0,
            _ => 0.25,
        };
        (result[i] * (4.0 * std::f32::consts::PI / SAMPLES as f32) * convolution / reference).extend(0.0).to_array()
    })
}

fn finite(value: f32, fallback: f32) -> f32 {
    if value.is_finite() {
        return value;
    }
    fallback
}

fn direction(i: usize, count: usize) -> Vec3 {
    let z = 1.0 - 2.0 * (i as f32 + 0.5) / count as f32;
    let angle = i as f32 * 2.3999631;
    let radius = (1.0 - z * z).sqrt();
    Vec3::new(radius * angle.cos(), radius * angle.sin(), z)
}

fn basis(n: Vec3) -> [f32; 9] {
    [
        0.282095,
        0.488603 * n.y,
        0.488603 * n.z,
        0.488603 * n.x,
        1.092548 * n.x * n.y,
        1.092548 * n.y * n.z,
        0.315392 * (3.0 * n.z * n.z - 1.0),
        1.092548 * n.x * n.z,
        0.546274 * (n.x * n.x - n.y * n.y),
    ]
}

fn smooth(lo: f32, hi: f32, value: f32) -> f32 {
    let t = ((value - lo) / (hi - lo)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn atmosphere(ray: Vec3, sun: Vec3, fog: Vec3, coverage: f32, warmth: f32) -> Vec3 {
    let mu = ray.dot(sun).clamp(-1.0, 1.0);
    let horizon = (1.0 - ray.z.max(0.0)).powi(5);
    let dusk = 1.0 - smooth(0.13, 0.42, sun.z);
    let extinction = (Vec3::new(-0.045, -0.10, -0.22) / (ray.z + 0.16).max(0.16)).exp();
    let sky =
        Vec3::new(0.24, 0.42, 0.72) * (Vec3::ONE - extinction) * Vec3::new(4.8, 3.6, 2.5) * 0.75 * (1.0 + mu * mu);
    let fog = fog.map(|v| {
        if v <= 0.04045 {
            return v / 12.92;
        }
        ((v + 0.055) / 1.055).powf(2.4)
    });
    let horizontal = Vec3::new(ray.x, ray.y, 0.001).normalize();
    let sunward = smooth(-0.35, 0.85, horizontal.dot(Vec3::new(sun.x, sun.y, 0.001).normalize()));
    let warm = fog.lerp(Vec3::new(0.82, 0.30, 0.09), (dusk * (0.30 + 0.55 * sunward) * warmth).clamp(0.0, 1.0));
    let colour = sky.lerp(warm, horizon * 0.85);
    let overcast = Vec3::splat(colour.dot(Vec3::new(0.24, 0.56, 0.20))) * Vec3::new(0.90, 0.97, 1.05);
    let colour = colour.lerp(overcast, coverage * 0.55);
    Vec3::new(0.008, 0.013, 0.027).lerp(colour, smooth(-0.12, 0.18, sun.z))
}

#[cfg(test)]
mod tests;
