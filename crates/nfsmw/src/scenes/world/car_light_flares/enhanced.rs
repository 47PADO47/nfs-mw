//! Optional headlamp cores and restrained halos at installed light markers.
use blackbox_gfx::{BlendMode, EffectLayer, PixelFormat, RenderBackend, TextureDesc, TextureHandle, TexturedEffect};
use glam::{Mat4, Vec3};

use super::FlareView;
use crate::scenes::world::car_lights::{Kind, Lamp};

const SIZE: u32 = 32;

fn pixels() -> Vec<u8> {
    let mut data = Vec::with_capacity((SIZE * SIZE * 4) as usize);
    for y in 0..SIZE {
        for x in 0..SIZE {
            let dx = (x as f32 + 0.5) * 2.0 / SIZE as f32 - 1.0;
            let dy = (y as f32 + 0.5) * 2.0 / SIZE as f32 - 1.0;
            let r2 = dx * dx + dy * dy;
            let alpha = ((1.0 - r2).max(0.0).powi(3) * 255.0).round() as u8;
            data.extend_from_slice(&[255, 255, 255, alpha]);
        }
    }
    data
}

pub(super) fn upload(renderer: &mut dyn RenderBackend) -> TextureHandle {
    let data = pixels();
    renderer.create_texture(&TextureDesc {
        label: "enhanced lamp glow",
        width: SIZE,
        height: SIZE,
        format: PixelFormat::Rgba8,
        mips: vec![&data],
    })
}

pub(super) fn draw<'a>(
    texture: TextureHandle,
    samples: impl Iterator<Item = (&'a Lamp, f32)> + Clone,
    frames: impl Iterator<Item = Mat4>,
    view: FlareView,
    focal: f32,
    out: &mut Vec<TexturedEffect>,
) {
    let right = view.forward.cross(Vec3::Z).normalize_or(Vec3::Y);
    let up = right.cross(view.forward).normalize_or(Vec3::Z);
    let mut vertices = Vec::new();
    for world in frames {
        for (lamp, intensity) in samples.clone() {
            if !matches!(lamp.kind, Kind::Head | Kind::Tail | Kind::CentreBrake) || intensity <= 0.0 {
                continue;
            }
            let at = world.transform_point3(lamp.at);
            let to_eye = view.camera - at;
            let depth = (-to_eye).dot(view.forward);
            let distance2 = to_eye.length_squared();
            if depth <= 0.3 || !depth.is_finite() || !distance2.is_finite() || distance2 >= 40000.0 {
                continue;
            }
            let facing = world.transform_vector3(lamp.direction).normalize_or_zero().dot(to_eye.normalize_or_zero());
            if facing <= 0.0 {
                continue;
            }
            let fade = ((40000.0 - distance2) / 8000.0).clamp(0.0, 1.0);
            let gain = (intensity * facing * facing * fade).clamp(0.0, 1.0);
            let center = at + to_eye.normalize_or_zero() * 0.06;
            let core = (depth / focal * 1.3).clamp(0.045, 0.15);
            if lamp.kind != Kind::Head {
                let x = right * core * 2.4;
                let y = up * core * 2.4;
                let color = [255, 28, 12, (30.0 * gain).round() as u8];
                EffectLayer::particle_quad(
                    &mut vertices,
                    [center - x + y, center + x + y, center + x - y, center - x - y],
                    color,
                    [0.0; 2],
                );
                continue;
            }
            for (radius, alpha) in [(core * 2.6, 38.0), (core, 190.0)] {
                let x = right * radius;
                let y = up * radius;
                let color = [255, 246, 230, (alpha * gain).round() as u8];
                EffectLayer::particle_quad(
                    &mut vertices,
                    [center - x + y, center + x + y, center + x - y, center - x - y],
                    color,
                    [0.0; 2],
                );
            }
        }
    }
    if !vertices.is_empty() {
        out.push(TexturedEffect { texture, blend: BlendMode::Additive, vertices });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glow_texture_has_a_soft_transparent_edge() {
        let rgba = pixels();
        assert_eq!(rgba[3], 0);
        let center = ((SIZE as usize / 2) * SIZE as usize + SIZE as usize / 2) * 4 + 3;
        assert!(rgba[center] > 240);
    }

    #[test]
    fn core_uses_head_state_and_faces_the_camera() {
        let lamp = Lamp { at: Vec3::ZERO, direction: Vec3::X, kind: Kind::Head, flags: 1 };
        let view = FlareView { camera: Vec3::X * 5.0, forward: -Vec3::X, fov_degrees: 60.0, height: 1080 };
        let texture = TextureHandle::from_raw(1);
        let render = |intensity: f32, direction: Vec3| {
            let lamp = Lamp { direction, ..lamp };
            let mut effects = Vec::new();
            draw(
                texture,
                std::iter::once((&lamp, intensity)),
                std::iter::once(Mat4::IDENTITY),
                view,
                935.0,
                &mut effects,
            );
            effects
        };
        assert_eq!(render(1.0, Vec3::X)[0].vertices.len(), 12);
        assert!(render(0.0, Vec3::X).is_empty());
        assert!(render(1.0, -Vec3::X).is_empty());
    }
}
