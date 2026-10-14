//! The blip under the cursor. The map attaches the cursor to the nearest blip of the current filter within a radius.

/// How far from the cursor, in view units (a full view width is 1.0), a blip still attaches.
pub const PICK_RADIUS: f32 = 0.06;

/// The index of the blip nearest to `cursor` (view units), within `radius`. A tie goes to the earlier blip.
/// `blips` are the positions of the blips already shown, in the same view units.
pub fn nearest(blips: &[[f32; 2]], cursor: [f32; 2], radius: f32) -> Option<usize> {
    let limit = radius * radius;
    let mut best: Option<(usize, f32)> = None;
    for (index, at) in blips.iter().enumerate() {
        let d = squared(*at, cursor);
        if d > limit {
            continue;
        }
        if best.is_none_or(|(_, b)| d < b) {
            best = Some((index, d));
        }
    }
    best.map(|(index, _)| index)
}

fn squared(a: [f32; 2], b: [f32; 2]) -> f32 {
    let dx = a[0] - b[0];
    let dy = a[1] - b[1];
    dx * dx + dy * dy
}
