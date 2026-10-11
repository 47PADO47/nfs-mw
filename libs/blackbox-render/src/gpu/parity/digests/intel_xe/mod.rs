//! Digests recorded on Intel Xe graphics (Iris Xe) with Vulkan and the Mesa driver.

mod bloom_aces;
mod direct;
mod fsr1_67;
mod fxaa;
mod offscreen;

/// The (scene, digest) pairs for a setting combination.
pub(super) fn table(combo: &str) -> Option<&'static [(&'static str, &'static str)]> {
    match combo {
        "direct" => Some(&direct::ALL),
        "offscreen" => Some(&offscreen::ALL),
        "fxaa" => Some(&fxaa::ALL),
        "bloom_aces" => Some(&bloom_aces::ALL),
        "fsr1_67" => Some(&fsr1_67::ALL),
        _ => None,
    }
}
