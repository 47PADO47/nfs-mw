//! The per-scene data every material reads: the frame's fog and light and the glossy lighting rig.
//!
//! It lives in one [`ShaderBuffer`] that every material binds as a read-only storage buffer, not in the materials.
//! The day-night clock changes it every frame, and a changed material is expensive in Bevy: a new bind group, and
//! every entity that uses it is re-extracted, re-specialised and re-binned. A changed `ShaderBuffer` of the same
//! size is rewritten in place on the GPU (`GpuShaderBuffer::prepare_asset`), so no material is touched.

use bevy_asset::{Assets, Handle, uuid_handle};
use bevy_render::render_resource::ShaderType;
use bevy_render::render_resource::encase::StorageBuffer;
use bevy_render::storage::ShaderBuffer;

use super::{Params, RigUniform};

/// The one buffer every [`super::BlackboxMaterial`] binds. A fixed id, so a material can name it without a store.
pub const SCENE_BUFFER: Handle<ShaderBuffer> = uuid_handle!("6f1b0f7e-52c4-4d0e-9b9a-3c1f5f0b7a21");

/// The buffer's contents; `Scene` in `blackbox.wesl`.
#[derive(Debug, Clone, Copy, PartialEq, Default, ShaderType)]
pub struct SceneUniform {
    pub params: Params,
    pub rig: RigUniform,
}

impl SceneUniform {
    fn bytes(&self) -> Vec<u8> {
        let mut buffer = StorageBuffer::new(Vec::new());
        buffer.write(self).expect("a growable Vec accepts any write");
        buffer.into_inner()
    }
}

/// Write `scene` into the shared buffer, creating it on first use.
pub fn write(buffers: &mut Assets<ShaderBuffer>, scene: &SceneUniform) {
    let mut buffer =
        buffers.get_or_insert_with(&SCENE_BUFFER, ShaderBuffer::default).expect("a UUID asset id is never invalid");
    buffer.clear();
    buffer.extend_from_slice(&scene.bytes());
}

#[cfg(test)]
mod tests {
    use bevy_math::Vec4;
    use bevy_render::render_resource::ShaderSize;

    use super::*;

    #[test]
    fn the_buffer_holds_the_scene_and_keeps_its_size() {
        let mut buffers = Assets::<ShaderBuffer>::default();
        let mut scene = SceneUniform::default();
        write(&mut buffers, &scene);
        let size = SceneUniform::SHADER_SIZE.get();
        assert_eq!(buffers.get(&SCENE_BUFFER).unwrap().buffer_size(), size);
        scene.params.fog_color = Vec4::new(0.1, 0.2, 0.3, 1.0);
        write(&mut buffers, &scene);
        let buffer = buffers.get(&SCENE_BUFFER).unwrap();
        // Same size, so the GPU buffer is rewritten in place and no bind group is rebuilt.
        assert_eq!(buffer.buffer_size(), size);
        let bytes = scene.bytes();
        assert_eq!(buffer.cast_slice::<u8>().unwrap(), bytes.as_slice());
        assert_eq!(&bytes[..4], &0.1f32.to_le_bytes());
    }
}
