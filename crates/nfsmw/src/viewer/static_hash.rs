//! A fingerprint of what a frame draws, to tell that nothing in the picture changed since the last frame (the front end's
//! still backdrop, `frontend/backdrop_still.rs`, and the draw step, `app/render/static_frame.rs`).

use std::hash::{Hash, Hasher};

use blackbox_gfx::{FrameParams, Instance, Projection};
use glam::{Mat4, Vec3};

/// Multiplier of the Fx hash. A change in any one word always changes the result, since every step is a bijection of
/// the running value.
const MIX: u64 = 0x517c_c1b7_2722_0a95;

/// A fast word-at-a-time hash of floats and integers. It is not meant to resist anyone, only to tell two frames apart.
#[derive(Debug, Clone, Copy)]
pub struct StaticHash(u64);

impl Default for StaticHash {
    fn default() -> Self {
        Self(0xcbf2_9ce4_8422_2325)
    }
}

impl Hasher for StaticHash {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, bytes: &[u8]) {
        let (words, rest) = bytes.as_chunks::<8>();
        for word in words {
            self.write_u64(u64::from_le_bytes(*word));
        }
        for &byte in rest {
            self.write_u64(u64::from(byte));
        }
    }

    fn write_u8(&mut self, value: u8) {
        self.write_u64(u64::from(value));
    }

    fn write_u32(&mut self, value: u32) {
        self.write_u64(u64::from(value));
    }

    fn write_u64(&mut self, value: u64) {
        self.0 = (self.0.rotate_left(5) ^ value).wrapping_mul(MIX);
    }

    fn write_usize(&mut self, value: usize) {
        self.write_u64(value as u64);
    }
}

impl StaticHash {
    pub fn f32(&mut self, value: f32) {
        self.write_u32(value.to_bits());
    }

    pub fn floats(&mut self, values: &[f32]) {
        values.iter().for_each(|&value| self.f32(value));
    }

    pub fn vec3(&mut self, value: Vec3) {
        self.floats(&value.to_array());
    }

    pub fn mat4(&mut self, value: &Mat4) {
        self.floats(&value.to_cols_array());
    }

    pub fn projection(&mut self, projection: &Projection) {
        match *projection {
            Projection::PerspectiveInfiniteReverse { fov_y, aspect, near } => {
                self.write_u8(1);
                self.floats(&[fov_y, aspect, near]);
            }
            Projection::Identity => self.write_u8(2),
        }
    }

    /// Everything of a frame's parameters that shapes the picture.
    pub fn params(&mut self, params: &FrameParams) {
        self.mat4(&params.view);
        self.projection(&params.projection);
        self.vec3(params.camera_position);
        self.vec3(params.light_dir);
        self.floats(&params.clear_color);
        match params.fog {
            Some(fog) => {
                self.write_u8(1);
                self.floats(&[fog.start, fog.end]);
            }
            None => self.write_u8(0),
        }
        match params.scattering {
            Some(s) => {
                self.write_u8(1);
                self.floats(&s.rayleigh);
                self.floats(&s.mie);
                self.vec3(s.to_sun);
                self.floats(&[s.g, s.distance_scale, s.in_scatter_scale]);
            }
            None => self.write_u8(0),
        }
        match params.surface_light {
            Some(l) => {
                self.write_u8(1);
                self.floats(&l.ambient);
                self.floats(&l.diffuse);
                self.floats(&l.specular);
                self.floats(&[
                    l.shadow_strength,
                    l.ambient_alpha,
                    l.specular_power,
                    l.reflection_scale,
                    l.reflective_specular_scale,
                ]);
            }
            None => self.write_u8(0),
        }
        self.write_u8(u8::from(params.camera_cut));
        match params.shadow_tuning {
            Some(tuning) => {
                self.write_u8(1);
                tuning.quality.hash(self);
                self.f32(tuning.bias);
                self.write_usize(tuning.divider);
            }
            None => self.write_u8(0),
        }
    }

    /// The instances in order: mesh, placement, identity and the flags that change how they draw.
    pub fn instances(&mut self, instances: &[Instance]) {
        self.write_usize(instances.len());
        for instance in instances {
            self.write_usize(instance.mesh.raw());
            self.mat4(&instance.transform);
            self.write_u64(instance.key.0);
            let flags = u8::from(instance.shadow.cast)
                | (u8::from(instance.shadow.receive) << 1)
                | (u8::from(instance.flip_winding) << 2);
            self.write_u8(flags);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use blackbox_gfx::MeshHandle;

    fn print(instances: &[Instance]) -> u64 {
        let mut hash = StaticHash::default();
        hash.instances(instances);
        hash.finish()
    }

    #[test]
    fn the_same_instances_hash_alike_and_any_change_shows() {
        let mesh = MeshHandle::from_raw(3);
        let a = [Instance::new(mesh, Mat4::IDENTITY)];
        assert_eq!(print(&a), print(&a));
        let moved = [Instance::new(mesh, Mat4::from_translation(Vec3::X * 0.001))];
        assert_ne!(print(&a), print(&moved));
        let other = [Instance::new(MeshHandle::from_raw(4), Mat4::IDENTITY)];
        assert_ne!(print(&a), print(&other));
        assert_ne!(print(&a), print(&[]));
    }
}
