//! The colours the flames are drawn in: the game's blue nitrous tint, and a mod variant's tint on top of it.

use blackbox_modding::content::FlameProfile;

/// The nitrous flame's colour with a mod variant's tint: each of red, green and blue is multiplied by the profile's, and the
/// alpha by its gain. Without a profile the colour is as it was.
pub(super) fn apply_flame_profile(color: [u8; 4], flame: Option<FlameProfile>) -> [u8; 4] {
    let Some(flame) = flame else { return color };
    let scale = |value: u8, factor: f32| (f32::from(value) * factor).clamp(0.0, 255.0) as u8;
    [
        scale(color[0], flame.tint[0]),
        scale(color[1], flame.tint[1]),
        scale(color[2], flame.tint[2]),
        scale(color[3], flame.gain),
    ]
}

/// The flame in blue: red and blue swap, so the fire keeps its shape and brightness and the nitrous reads cold.
pub(super) fn tint_blue(color: [u8; 4]) -> [u8; 4] {
    [color[2], color[1], color[0], color[3]]
}
