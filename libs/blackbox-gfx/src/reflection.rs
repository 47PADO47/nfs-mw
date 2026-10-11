//! Local environment captures, in the same game axes as glossy reflection sampling.

use glam::Vec3;

use crate::{FrameParams, Instance, Projection};

/// Capture resolution. All six faces are refreshed together; no temporal accumulation.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum ReflectionQuality {
    #[default]
    Off,
    Low,
    Medium,
    High,
}

impl ReflectionQuality {
    pub const ALL: [Self; 4] = [Self::Off, Self::Low, Self::Medium, Self::High];

    pub const fn name(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
        }
    }

    pub const fn size(self) -> u32 {
        match self {
            Self::Off => 0,
            Self::Low => 64,
            Self::Medium => 128,
            Self::High => 256,
        }
    }
}

impl std::str::FromStr for ReflectionQuality {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|q| q.name() == s.trim().to_ascii_lowercase())
            .ok_or_else(|| format!("expected off, low, medium or high, got {s:?}"))
    }
}

impl std::fmt::Display for ReflectionQuality {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

/// One surroundings snapshot. The caller supplies scenery outside the main view too, and excludes
/// the reflective object itself. Effects and UI are deliberately absent from this pass.
pub struct ReflectionScene<'a> {
    pub origin: Vec3,
    pub quality: ReflectionQuality,
    pub frame: FrameParams,
    /// Frustum/LOD-selected scenery for each face, including objects behind the main camera.
    pub faces: [&'a [Instance]; 6],
}

/// RH face cameras (+X, -X, +Y, -Y, +Z, -Z). A capture must flip the image vertically
/// when storing it: standard cube addressing has the opposite handedness to a RH camera.
pub fn face_frame(origin: Vec3, face: usize, template: &FrameParams) -> FrameParams {
    let directions = [Vec3::X, Vec3::NEG_X, Vec3::Y, Vec3::NEG_Y, Vec3::Z, Vec3::NEG_Z];
    let ups = [Vec3::NEG_Y, Vec3::NEG_Y, Vec3::Z, Vec3::NEG_Z, Vec3::NEG_Y, Vec3::NEG_Y];
    FrameParams {
        view: glam::camera::rh::view::look_at_mat4(origin, origin + directions[face], ups[face]),
        projection: Projection::PerspectiveInfiniteReverse {
            fov_y: std::f32::consts::FRAC_PI_2,
            aspect: 1.0,
            near: 0.25,
        },
        camera_position: origin,
        camera_cut: true,
        ..*template
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn face_cameras_agree_with_cube_texel_addressing() {
        let frame = FrameParams {
            view: glam::Mat4::IDENTITY,
            projection: Projection::Identity,
            camera_position: Vec3::ZERO,
            light_dir: Vec3::Z,
            clear_color: [0.0; 3],
            fog: None,
            camera_cut: false,
        };
        for (face, direction) in
            [Vec3::X, Vec3::NEG_X, Vec3::Y, Vec3::NEG_Y, Vec3::Z, Vec3::NEG_Z].into_iter().enumerate()
        {
            let f = face_frame(Vec3::new(4.0, 8.0, 2.0), face, &frame);
            let camera = f.view.inverse();
            assert!(camera.transform_vector3(Vec3::NEG_Z).abs_diff_eq(direction, 1e-6));
            // Cube sampling at (s=.75,t=.25) must project to that pixel on every face.
            let expected = match face {
                0 => Vec3::new(1.0, 0.5, -0.5),
                1 => Vec3::new(-1.0, 0.5, 0.5),
                2 => Vec3::new(0.5, 1.0, -0.5),
                3 => Vec3::new(0.5, -1.0, 0.5),
                4 => Vec3::new(0.5, 0.5, 1.0),
                _ => Vec3::new(-0.5, 0.5, -1.0),
            };
            let clip = f.view_proj() * (f.camera_position + expected).extend(1.0);
            assert!((clip.x / clip.w - 0.5).abs() < 1e-5, "face {face}");
            assert!((clip.y / clip.w + 0.5).abs() < 1e-5, "face {face}");
        }
    }
}
