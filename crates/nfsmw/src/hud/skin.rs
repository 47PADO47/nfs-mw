//! Names of the textures the tachometer swaps in for a skin (`GLOBAL/HUDS_Custom_NN.bin`).

use blackbox_feng::fe_hash_upper;

/// The tachometer face texture for a scale ending at `max_rpm`: `7000_LINES_NN` … `10000_LINES_NN`.
pub fn tach_face_texture(max_rpm: f32, skin: u8) -> u32 {
    let n = ((max_rpm / 1000.0).ceil() as i32 * 1000).clamp(7000, 10000);
    fe_hash_upper(&format!("{n}_LINES_{skin:02}"))
}

/// The needle texture of a skin.
pub fn needle_texture(skin: u8) -> u32 {
    fe_hash_upper(&format!("TACH_NEEDLE_{skin:02}"))
}

/// The tachometer fill texture of a skin.
pub fn fill_texture(skin: u8) -> u32 {
    fe_hash_upper(&format!("TACH_FILL_{skin:02}"))
}
