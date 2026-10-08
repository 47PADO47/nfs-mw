//! Names of the textures the tachometer swaps in for a skin (`GLOBAL/HUDS_Custom_NN.bin`).

use blackbox_feng::fe_hash_upper;

use super::state::scale_rpm;

/// The tachometer face texture for an engine with this `MAX_RPM`: `7000_LINES_NN` … `10000_LINES_NN`.
pub fn tach_face_texture(max_rpm: f32, skin: u8) -> u32 {
    let n = scale_rpm(max_rpm) as u32;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_face_follows_the_scale() {
        assert_eq!(tach_face_texture(8250.0, 0), fe_hash_upper("9000_LINES_00"));
        assert_eq!(tach_face_texture(6500.0, 3), fe_hash_upper("7000_LINES_03"));
        assert_eq!(tach_face_texture(11000.0, 0), fe_hash_upper("10000_LINES_00"));
    }
}
