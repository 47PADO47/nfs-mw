//! The screen side of the 2012 nitrous as the `[screen]` section of the variant's profile (`docs/modding/nitrous-variants.md`,
//! evidence in `docs/formats/chameleon-nitrous.md`, "Post effects").
//!
//! `POSTFX.BNDL` holds a state bound to `Reporter.NitrousState.UsingNitrous`; its imports name the keyframe (vignette, bloom,
//! general) and the colour cube it blends in while the nitrous burns, and the state holds the seconds it takes. Read with
//! `chameleon-postfx`, the game gets:
//!
//! - the darkening vignette: the keyframe's outer colour is what the screen edge goes to (a dark teal), and its second pair of
//!   numbers is read as where the vignette starts;
//! - the grade: the colour cube's grey diagonal fitted as a tint and a contrast;
//! - the fades: the state's blend time, for the way in and the way out.
//!
//! The bloom tint, the depth of field and the stereo part have no control in the renderer (the bloom is the player's own post
//! setting). The 2012 data holds nothing for the field of view, the camera shake, the pull-back or the radial blur of the
//! nitrous (the camera is the executable's, not read), so those five values are the rewrite's own, chosen by eye against the
//! 2005 game's camera: small, because the 2005 nitrous already widens the view and blurs.

use chameleon_bundle::Bundle;
use chameleon_postfx::{Bound, find_bound};

use crate::source::Source;

/// The file with the post effect states.
pub const POST_FX: &str = "POSTFX.BNDL";
/// The live value that turns the nitrous state on.
const NITROUS_EXPRESSION: &str = "NitrousState.UsingNitrous";

/// The rewrite's own camera values at full effect (the 2012 data has none): degrees of field of view on top of the 2005
/// nitrous kick, metres of pull-back, the size of the shake in metres and its rate, and the radial blur.
const FOV: f32 = 4.0;
const PULL_BACK: f32 = 0.35;
const SHAKE: f32 = 0.006;
const SHAKE_RATE: f32 = 22.0;
const RADIAL_BLUR: f32 = 0.25;

/// What the 2012 data gives for the screen of the nitrous: the state, its keyframe and its colour cube.
pub type Screen = Bound;

/// The nitrous screen of the install, from its `POSTFX.BNDL`.
pub fn find(source: &Source) -> Option<Screen> {
    let bytes = source.read_file(POST_FX).map_err(|error| log::warn!("nitrous screen: {error:#}")).ok()?;
    let bundle = Bundle::parse(&bytes).map_err(|error| log::warn!("nitrous screen: {POST_FX}: {error}")).ok()?;
    find_bound(&bundle, NITROUS_EXPRESSION)
}

fn list(values: [f32; 3]) -> String {
    format!("[{:.4}, {:.4}, {:.4}]", values[0], values[1], values[2])
}

/// The `[screen]` section for `screen`, ending in a new line.
pub fn section(screen: &Screen) -> String {
    let outer = screen.keyframe.vignette.outer;
    let [bx, by] = screen.keyframe.vignette.b;
    let mut text = String::from("\n[screen]\n");
    text.push_str(
        "# darkening vignette: the 2012 keyframe's outer colour, starting where its second pair of numbers says\n",
    );
    text.push_str(&format!("edge = {}\n", list([outer[0], outer[1], outer[2]].map(|c| c.clamp(0.0, 4.0)))));
    text.push_str(&format!("edge_inner = {:.4}\n", ((bx + by) * 0.5).clamp(0.0, 1.0)));
    if let Some(cube) = screen.cube.as_ref().filter(|cube| !cube.is_identity()) {
        let grade = cube.grade();
        text.push_str("# grade: the colour cube's grey diagonal fitted as a tint and a contrast\n");
        text.push_str(&format!("tint = {}\n", list(grade.tint.map(|c| c.clamp(0.0, 4.0)))));
        text.push_str(&format!("contrast = {:.4}\n", grade.contrast.clamp(0.0, 4.0)));
    }
    let fade = screen.state.blend_seconds.clamp(0.0, 10.0);
    text.push_str(&format!("fade_in = {fade:.3}\nfade_out = {fade:.3}\n"));
    text.push_str("# the rewrite's own: the 2012 data has no camera or blur values for the nitrous\n");
    text.push_str(&format!("radial_blur = {RADIAL_BLUR:?}\nfov = {FOV:?}\npull_back = {PULL_BACK:?}\n"));
    text.push_str(&format!("shake = {SHAKE:?}\nshake_rate = {SHAKE_RATE:?}\n"));
    text
}

#[cfg(test)]
mod tests;
