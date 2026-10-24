//! The font files tried on Windows. Bahnschrift, shipped with Windows 10 and later, is a variable DIN face with a width
//! axis (`wdth`, 75 to 100) and a weight axis (`wght`, 300 to 700), so one file gives every role its own instance. Older
//! systems fall back to Arial Narrow for numbers (when installed) and to Segoe UI.

use std::path::Path;

use super::{Axis, Candidate};

/// Bahnschrift for numbers: condensed and semi-bold.
pub const NUMERIC_AXES: &[Axis] = &[(*b"wdth", 75.0), (*b"wght", 600.0)];
/// Bahnschrift for labels: semi-condensed, regular.
pub const LABEL_AXES: &[Axis] = &[(*b"wdth", 87.5), (*b"wght", 400.0)];
/// Bahnschrift for spaced captions: full width, semi-light.
pub const LABEL_WIDE_AXES: &[Axis] = &[(*b"wdth", 100.0), (*b"wght", 350.0)];

/// The candidates of each role, in `Role::ALL` order, in the font folder `fonts`.
pub fn candidates(fonts: &Path) -> [Vec<Candidate>; 3] {
    let bahnschrift = |axes: &'static [Axis]| Candidate::new(fonts.join("bahnschrift.ttf"), 0, axes);
    let file = |name: &str| Candidate::new(fonts.join(name), 0, &[]);
    [
        vec![bahnschrift(NUMERIC_AXES), file("arialnb.ttf"), file("arialn.ttf"), file("segoeuib.ttf")],
        vec![bahnschrift(LABEL_AXES), file("segoeui.ttf")],
        vec![bahnschrift(LABEL_WIDE_AXES), file("segoeui.ttf")],
    ]
}
