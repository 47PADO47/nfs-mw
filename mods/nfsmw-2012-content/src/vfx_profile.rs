//! The `[[effect]]` entries and the pictures of the 2012 nitrous variant, made from the emitters of the three 2012 effects
//! (`vfx`).
//!
//! What is read from the 2012 data: which emitters each effect has, their pictures (the flame sheet, the glow, the dust), that a
//! picture is a flipbook (a `SubUV` attribute), its cell grid (the flame sheet is 5 by 5 cells, seen on the picture) and the
//! glow's colour constant. What is **not** read, because the emitters' programs are not decoded: how often they spawn, how long
//! they live, how fast they move and how big they get. Those numbers are this module's own, set by eye against screenshots of
//! the 2012 game (an orange burst at the pipes when the nitrous lights, two blue glows while it burns, a short flame at a
//! pop), and the docs say so.
//!
//! Emitters with no flipbook and no colour of their own in a start or pop effect (the three smoke puffs) are not played.

use crate::vfx::{Emitter, Picture};

/// The files and the text a variant's effects need.
#[derive(Debug, Clone, PartialEq)]
pub struct EffectSet {
    /// File name and picture, written under `files/NITROUS/MW2012/`.
    pub pictures: Vec<(&'static str, Picture)>,
    /// The `[[effect]]` tables, and whether there are burn and pop effects (so the game's own flames can be replaced).
    pub toml: String,
    pub has_burn: bool,
    pub has_pop: bool,
}

/// The folder the pictures are named in.
const FOLDER: &str = "NITROUS/MW2012";
/// The cells per side of the flame sheet (25 frames).
const FLAME_GRID: u8 = 5;
/// A flipbook picture at least this wide is the flame sheet; a smaller one is the dust.
const FLAME_WIDTH: u32 = 512;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Look {
    Flame,
    Dust,
}

fn look(emitter: &Emitter) -> Option<Look> {
    match (emitter.flipbook, emitter.picture.width >= FLAME_WIDTH) {
        (true, true) => Some(Look::Flame),
        (true, false) => Some(Look::Dust),
        (false, _) => None,
    }
}

/// The effects of the three moments, from the emitters of the 2012 effects. `None` when no emitter gives anything to play.
pub fn build(start: &[Emitter], burn: &[Emitter], pop: &[Emitter]) -> Option<EffectSet> {
    let mut set = EffectSet { pictures: Vec::new(), toml: String::new(), has_burn: false, has_pop: false };
    for (on, emitters) in [("start", start), ("pop", pop)] {
        let mut flames = 0;
        for emitter in emitters {
            let Some(kind) = look(emitter) else { continue };
            let name = add_picture(&mut set, kind, &emitter.picture);
            let entry = match kind {
                Look::Flame => flame(on, name, flames),
                Look::Dust => dust(on, name),
            };
            flames += usize::from(kind == Look::Flame);
            set.toml.push_str(&entry);
            set.has_pop |= on == "pop";
        }
    }
    // The glow is the burn's one emitter without a flipbook; its colour is the one read from the effect.
    if let Some(glow) = burn.iter().find(|emitter| !emitter.flipbook) {
        set.pictures.push(("GLOW.PNG", glow.picture.clone()));
        let colour = glow.colour.unwrap_or([0.03, 0.52, 0.86]);
        set.toml.push_str(&glow_halo(colour));
        set.toml.push_str(&glow_core(colour));
        set.has_burn = true;
    }
    (!set.toml.is_empty()).then_some(set)
}

/// The file name of `picture` of `kind`, adding it once.
fn add_picture(set: &mut EffectSet, kind: Look, picture: &Picture) -> &'static str {
    let name = match kind {
        Look::Flame => "FLAME.PNG",
        Look::Dust => "DUST.PNG",
    };
    if !set.pictures.iter().any(|(known, _)| *known == name) {
        set.pictures.push((name, picture.clone()));
    }
    name
}

fn path(name: &str) -> String {
    format!("{FOLDER}/{name}")
}

/// A flame burst: the sheet played once over a particle's life, leaving the pipe along its axis. The second flame of an effect
/// is a little smaller and slower, as the 2012 effects have two.
fn flame(on: &str, name: &str, nth: usize) -> String {
    let (life, speed, scale) = match (on, nth) {
        ("start", 0) => (0.35, 3.0, 1.0),
        ("start", _) => (0.30, 1.8, 0.7),
        (_, 0) => (0.22, 4.0, 0.8),
        _ => (0.18, 2.4, 0.55),
    };
    let rate = if on == "start" { 25.0 } else { 45.0 };
    let duration = if on == "start" { "duration = 0.8\n" } else { "" };
    let fps = (f32::from(FLAME_GRID).powi(2) / life).round();
    format!(
        "\n[[effect]]\non = \"{on}\"\ntexture = \"{}\"\nblend = \"additive\"\ngrid = {FLAME_GRID}\nfps = {fps}\nrate = {rate:?}\nlife = {life:?}\n\
         life_variance = 0.05\nspeed = {speed:?}\nspeed_variance = 0.5\nspread = 18.0\ninherit = 0.3\n\
         size = [{:.2}, {:.2}, {:.2}, {:.2}]\ncolors = [[255, 255, 255, 0], [255, 255, 255, 235], [255, 255, 255, 200], [255, 255, 255, 0]]\n{duration}",
        path(name),
        0.25 * scale,
        0.45 * scale,
        0.55 * scale,
        0.6 * scale,
    )
}

/// The dust and sparks round the pipes: the small picture, drifting out and fading.
fn dust(on: &str, name: &str) -> String {
    let duration = if on == "start" { "duration = 0.8\n" } else { "" };
    format!(
        "\n[[effect]]\non = \"{on}\"\ntexture = \"{}\"\nblend = \"additive\"\nrate = 30.0\nlife = 0.5\nlife_variance = 0.2\n\
         speed = 1.5\nspeed_variance = 1.0\nspread = 120.0\ninherit = 0.2\nsize = [0.04, 0.07, 0.08, 0.05]\n\
         colors = [[255, 200, 120, 0], [255, 200, 120, 200], [255, 160, 80, 140], [255, 120, 60, 0]]\n{duration}",
        path(name)
    )
}

fn byte(c: f32) -> u8 {
    (c.clamp(0.0, 1.0) * 255.0).round() as u8
}

/// The glow at a pipe while the nitrous burns: the picture in the colour read from the 2012 effect, held on the pipe.
fn glow_halo(colour: [f32; 3]) -> String {
    let [r, g, b] = colour.map(byte);
    format!(
        "\n[[effect]]\non = \"burn\"\ntexture = \"{}\"\nblend = \"additive\"\nrate = 40.0\nlife = 0.12\nspeed = 0.0\ninherit = 1.0\n\
         size = [0.14, 0.32, 0.32, 0.18]\ncolors = [[{r}, {g}, {b}, 0], [{r}, {g}, {b}, 230], [{r}, {g}, {b}, 230], [{r}, {g}, {b}, 0]]\n",
        path("GLOW.PNG")
    )
}

/// The brighter, smaller core of the glow (the screenshots show a white-blue centre), a lighter mix of the same colour.
fn glow_core(colour: [f32; 3]) -> String {
    let [r, g, b] = colour.map(|c| byte(c + (1.0 - c) * 0.7));
    format!(
        "\n[[effect]]\non = \"burn\"\ntexture = \"{}\"\nblend = \"additive\"\nrate = 40.0\nlife = 0.12\nspeed = 0.0\ninherit = 1.0\n\
         size = [0.06, 0.13, 0.13, 0.07]\ncolors = [[{r}, {g}, {b}, 0], [{r}, {g}, {b}, 255], [{r}, {g}, {b}, 255], [{r}, {g}, {b}, 0]]\n",
        path("GLOW.PNG")
    )
}

#[cfg(test)]
mod tests;
