//! The environment cube map: face layout and the procedural sky.

use glam::Vec3;

use crate::SkyGradient;

/// Faces in the order GPUs expect: +x, -x, +y, -y, +z, -z.
pub(super) const FACES: usize = 6;

/// The direction a texel of cube `face` looks at; `s` and `t` run 0 to 1 across the face
/// (left to right, top to bottom in the image). Standard cube-map addressing.
pub(super) fn face_direction(face: usize, s: f32, t: f32) -> Vec3 {
    let (sc, tc) = (2.0 * s - 1.0, 2.0 * t - 1.0);
    let d = match face {
        0 => Vec3::new(1.0, -tc, -sc),
        1 => Vec3::new(-1.0, -tc, sc),
        2 => Vec3::new(sc, 1.0, tc),
        3 => Vec3::new(sc, -1.0, -tc),
        4 => Vec3::new(sc, -tc, 1.0),
        _ => Vec3::new(-sc, -tc, -1.0),
    };
    d.normalize()
}

fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// The sky colour looking along unit `dir` (+z is up).
pub(super) fn sky_color(sky: &SkyGradient, dir: Vec3) -> Vec3 {
    let [horizon, zenith, ground] = [sky.horizon, sky.zenith, sky.ground].map(Vec3::from);
    if dir.z >= 0.0 {
        return horizon.lerp(zenith, dir.z.powf(0.6));
    }
    horizon.lerp(ground, smoothstep(0.0, 0.35, -dir.z))
}

/// The six RGBA8 faces (`size` × `size` texels each) of `sky`.
pub(super) fn sky_faces(sky: &SkyGradient, size: u32) -> [Vec<u8>; FACES] {
    std::array::from_fn(|face| {
        let mut texels = Vec::with_capacity((size * size * 4) as usize);
        for y in 0..size {
            for x in 0..size {
                let (s, t) = ((x as f32 + 0.5) / size as f32, (y as f32 + 0.5) / size as f32);
                let rgb = sky_color(sky, face_direction(face, s, t));
                texels.extend(rgb.to_array().map(|c| (c.clamp(0.0, 1.0) * 255.0 + 0.5) as u8));
                texels.push(255);
            }
        }
        texels
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn face_centres_look_along_the_axes() {
        let axes = [Vec3::X, Vec3::NEG_X, Vec3::Y, Vec3::NEG_Y, Vec3::Z, Vec3::NEG_Z];
        for (face, axis) in axes.into_iter().enumerate() {
            assert!((face_direction(face, 0.5, 0.5) - axis).length() < 1e-6, "face {face}");
        }
    }

    #[test]
    fn top_face_is_zenith_and_bottom_face_is_ground() {
        let sky = SkyGradient::default();
        let faces = sky_faces(&sky, 9);
        let centre = |face: usize| {
            let at = (4 * 9 + 4) * 4;
            [faces[face][at], faces[face][at + 1], faces[face][at + 2], faces[face][at + 3]]
        };
        let byte = |c: f32| (c * 255.0 + 0.5) as u8;
        assert_eq!(centre(4), [byte(sky.zenith[0]), byte(sky.zenith[1]), byte(sky.zenith[2]), 255]);
        assert_eq!(centre(5), [byte(sky.ground[0]), byte(sky.ground[1]), byte(sky.ground[2]), 255]);
    }

    #[test]
    fn gradient_runs_from_ground_through_horizon_to_zenith() {
        let sky = SkyGradient::default();
        assert_eq!(sky_color(&sky, Vec3::Z), Vec3::from(sky.zenith));
        assert_eq!(sky_color(&sky, Vec3::X), Vec3::from(sky.horizon));
        assert_eq!(sky_color(&sky, Vec3::NEG_Z), Vec3::from(sky.ground));
        let halfway = sky_color(&sky, Vec3::new(1.0, 0.0, 0.5).normalize());
        let (horizon, zenith) = (Vec3::from(sky.horizon), Vec3::from(sky.zenith));
        assert!(halfway.cmpge(horizon.min(zenith) - 1e-6).all() && halfway.cmple(horizon.max(zenith) + 1e-6).all());
        assert_ne!(halfway, horizon);
    }

    #[test]
    fn side_face_centres_are_horizon() {
        let sky = SkyGradient::default();
        let faces = sky_faces(&sky, 9);
        let at = (4 * 9 + 4) * 4;
        let horizon = sky.horizon.map(|c| (c * 255.0 + 0.5) as u8);
        for (face, texels) in faces.iter().enumerate().take(4) {
            assert_eq!(texels[at..at + 3], horizon, "face {face}");
        }
    }
}
