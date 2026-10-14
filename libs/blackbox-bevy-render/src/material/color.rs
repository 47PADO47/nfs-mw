//! Bridge display-authored game colours into Bevy's scene-referred ACES range.
//!
//! The game shader has already shaped its output for a clamped SDR display. Applying an HDR curve
//! directly to its sRGB decode crushes shadows. Calibrate the neutral axis against Bevy's fitted
//! RRT/ODT, with reference white at 90% display-linear output to leave highlight headroom. This is an
//! artistic SDR-to-HDR adapter, not recovered physical lighting. `blackbox.wesl` uses the same transfer.

use blackbox_gfx::Tonemap;

pub fn scene_rgb(gamma: [f32; 3], tonemap: Tonemap) -> [f32; 3] {
    gamma.map(|c| {
        let linear = match c <= 0.04045 {
            true => c / 12.92,
            false => ((c + 0.055) / 1.055).powf(2.4),
        };
        if tonemap != Tonemap::Aces {
            return linear;
        }
        aces_reference(linear)
    })
}

fn aces_reference(linear: f32) -> f32 {
    if linear <= 0.0 {
        return 0.0;
    }
    let y = linear.min(1.0) * 0.9;
    let a = 1.0 - 0.983729 * y;
    let b = 0.0245786 - 0.432951 * y;
    let c = 0.000090537 + 0.238081 * y;
    let reference = (-b + (b * b + 4.0 * a * c).sqrt()) / (2.0 * a);
    reference + (linear - 1.0).max(0.0) * reference
}
