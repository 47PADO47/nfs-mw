//! The 2012 car's corona objects as the sprites of a mod car's lamps: `[[lamp_flare]]` tables for the head, tail, brake and
//! reverse lamps and `[[light_bar]]` tables for the siren lamps of a police car (`docs/modding/car-lamps.md`,
//! `docs/modding/cop-cars.md`; the data: `docs/formats/chameleon-corona.md`).
//!
//! Every corona record of the car's bundle names a locator group (where), a bit of the light-state word (when) and a `Corona`
//! object (what it looks like). The object's glow and flare layers are evaluated at a near and a far distance
//! (`chameleon_effects::corona`) and carried over as a sprite with a radius and a brightness at those two distances; the
//! sprite picture is the layer's own, converted into the car's texture pack. A siren record also names a flash pattern, which is
//! baked into the digit string of its `[[light_bar]]`. The look is an approximation of the 2012 game's: the executable's units
//! and its brightness scale are not known, so the brightness is relative (the brightest of the two distances is 1) and the radius
//! is taken from the sprite's two sizes.

mod pattern;
mod sprites;
#[cfg(test)]
mod tests;

use chameleon_bundle::Bundle;
use chameleon_effects::corona::pattern::FlashPattern;
use chameleon_effects::corona::{Corona, Layer, LayerKind, RecordFactors, Reference};
use chameleon_gfx::Texture;
use chameleon_vehicle::{CoronaRecord, Lamps, read_corona_records};

use crate::markers::distance;
use sprites::Sprites;

/// The near and the far distance, metres, the sprites are given at.
pub const NEAR: f32 = 8.0;
pub const FAR: f32 = 100.0;
/// The manifest's limits on lamps.
const MAX_FLARES: usize = 96;
const MAX_BARS: usize = 32;
/// Most layers of one corona that become sprites.
const MAX_LAYERS: usize = 2;
/// Two lamps of one kind closer than this, metres, are one lamp.
const SAME_PLACE: f32 = 0.02;
/// A lamp this close to the centre plane, metres, is a centre lamp.
const CENTRE_BAND: f32 = 0.05;
/// The radius limits of a sprite, metres.
const RADIUS: (f32, f32) = (0.03, 2.0);
/// A siren's state bit.
const SIREN_BIT: u16 = 3;
/// The files of the install the pictures are in.
const MATERIALS: &str = "GLOBALMATERIALDICTIONARY.BNDL";
const TEXTURES: &str = "GLOBALTEXTUREDICTIONARY.BNDL";

/// What a car's coronas became: the pictures to add to its pack and the manifest tables.
pub struct Coronas {
    /// Each picture and the name it takes in the car's `TEXTURES.BIN`.
    pub sprites: Vec<(String, Texture)>,
    /// The tables, starting with a newline; empty when no lamp was carried over.
    pub manifest: String,
    /// How many flares and light bars were written (the tests count them).
    #[cfg_attr(not(test), allow(dead_code))]
    pub flares: usize,
    #[cfg_attr(not(test), allow(dead_code))]
    pub bars: usize,
}

/// The look a flare and a siren lamp share, in the manifest's terms.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Look {
    pub sprite: String,
    pub position: [f32; 3],
    pub direction: [f32; 3],
    pub colour: [f32; 3],
    pub alpha: [f32; 2],
    pub radius: [f32; 2],
    pub falloff: f32,
}

/// One lamp of the manifest.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Row {
    Flare { lit: &'static str, side: &'static str, look: Look },
    Bar { siren: &'static str, look: Look, pattern: String, step: f32 },
}

/// Carry the coronas of the car with the bundle `bytes` over. `to_car` turns a point of the 2012 vehicle space into the car's
/// space. `None` when the car has none that can be drawn.
pub fn build(
    install: &dyn Fn(&str) -> Option<Vec<u8>>,
    bytes: &[u8],
    lamps: &Lamps,
    to_car: &dyn Fn([f32; 3]) -> [f32; 3],
    folder: &str,
) -> Option<Coronas> {
    let bundle = Bundle::parse(bytes).ok()?;
    let records = read_corona_records(&bundle).ok()?;
    let (materials, textures) = (install(MATERIALS)?, install(TEXTURES)?);
    let mut sprites = Sprites::open(&materials, &textures, folder)?;
    let corona_of = |id: u64| {
        let resource = bundle.resource(bundle.find(id)?).ok()?;
        let refs: Vec<Reference> =
            resource.imports.iter().map(|import| Reference { at: import.slot() as usize, id: import.id }).collect();
        Corona::parse(&resource.blocks[0], &refs).ok()
    };
    let flash_of = |id: u64| pattern::read(&bundle, id);
    let rows = rows(&records, lamps, to_car, &corona_of, &flash_of, &mut |material| sprites.name(material));
    let (flares, bars) = count(&rows);
    let manifest = manifest(&rows);
    (!rows.is_empty()).then(|| Coronas { sprites: sprites.finish(), manifest, flares, bars })
}

fn count(rows: &[Row]) -> (usize, usize) {
    let bars = rows.iter().filter(|row| matches!(row, Row::Bar { .. })).count();
    (rows.len() - bars, bars)
}

/// What a record lights: a lamp kind of the manifest, or a siren.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Role {
    Lamp { lit: &'static str, front: bool },
    Siren,
}

/// The role of a record, from its light-state bit and the end of the car it sits at (`docs/formats/chameleon-lamps.md`).
/// The fog lamps and the second switch of an ordinary car have none.
fn role(record: &CoronaRecord, front: bool) -> Option<Role> {
    if record.state_bit == SIREN_BIT && record.flash.is_some() {
        return Some(Role::Siren);
    }
    match (record.state_bit, front) {
        (0 | 4 | 6 | 7, true) => Some(Role::Lamp { lit: "head", front }),
        (1, false) => Some(Role::Lamp { lit: "brake", front }),
        (0 | 7, false) => Some(Role::Lamp { lit: "tail", front }),
        (2, false) => Some(Role::Lamp { lit: "reverse", front }),
        _ => None,
    }
}

/// A place of a lamp in the car's space and the way it shines.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Place {
    at: [f32; 3],
    shines: [f32; 3],
}

/// A unit vector along `v`, turned to the end of the car the lamp belongs to when it points the wrong way or is too flat.
fn unit(v: [f32; 3], front: bool) -> [f32; 3] {
    let length = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    let end = match front {
        true => 1.0,
        false => -1.0,
    };
    if length < 1e-6 || v[0] * end / length < 0.5 {
        return [end, 0.0, 0.0];
    }
    [v[0] / length, v[1] / length, v[2] / length]
}

/// The places of a record's lamps: each locator of its group, and its mirror image across the centre plane for a lamp of
/// the car's ends (the mode of a record is not decoded far enough to tell, `chameleon-lamps.md`) or a siren of mode 2.
fn places(lamps: &Lamps, record: &CoronaRecord, role: Role, to_car: &dyn Fn([f32; 3]) -> [f32; 3]) -> Vec<Place> {
    let Some(group) = lamps.group(record.group) else { return Vec::new() };
    let mirrored = match role {
        Role::Lamp { .. } => true,
        Role::Siren => record.mode == 2,
    };
    let mut found: Vec<Place> = Vec::new();
    for locator in &group.locators {
        let front = locator.position[2] > 0.0;
        if matches!(role, Role::Lamp { front: wanted, .. } if wanted != front) {
            continue;
        }
        let (turned, origin) = (to_car(locator.forward()), to_car([0.0; 3]));
        let shines = unit([turned[0] - origin[0], turned[1] - origin[1], turned[2] - origin[2]], front);
        let at = to_car(locator.position);
        let sides: &[f32] = match mirrored {
            true => &[1.0, -1.0],
            false => &[1.0],
        };
        for &side in sides {
            let place = Place { at: [at[0], side * at[1], at[2]], shines: [shines[0], side * shines[1], shines[2]] };
            if !found.iter().any(|known| distance(known.at, place.at) < SAME_PLACE) {
                found.push(place);
            }
        }
    }
    found
}

/// The side of the car a place is on.
fn side_of(y: f32) -> &'static str {
    match y {
        y if y > CENTRE_BAND => "left",
        y if y < -CENTRE_BAND => "right",
        _ => "centre",
    }
}

/// The manifest rows of the records. `corona_of` and `flash_of` read the objects a record names, `sprite_of` gives the name of
/// a layer's picture in the car's pack.
pub(crate) fn rows(
    records: &[CoronaRecord],
    lamps: &Lamps,
    to_car: &dyn Fn([f32; 3]) -> [f32; 3],
    corona_of: &dyn Fn(u64) -> Option<Corona>,
    flash_of: &dyn Fn(u64) -> Option<FlashPattern>,
    sprite_of: &mut dyn FnMut(u64) -> Option<String>,
) -> Vec<Row> {
    let mut out: Vec<Row> = Vec::new();
    for record in records {
        let Some(corona) = record.corona.and_then(corona_of) else { continue };
        let Some(first) = lamps.group(record.group).and_then(|group| group.locators.first()) else { continue };
        let Some(role) = role(record, first.position[2] > 0.0) else { continue };
        let power = corona.falloff_power();
        // A lamp that cannot be seen on its axis (a beam) has no flare; a siren is seen from every side.
        let falloff = match (role, power) {
            (Role::Siren, _) => 0.0,
            (_, Some(power)) => power,
            (_, None) => continue,
        };
        let layers = sprites_of(&corona, record, &mut *sprite_of);
        if layers.is_empty() {
            continue;
        }
        let pattern = match role {
            Role::Siren => record.flash.and_then(flash_of).map(|flash| pattern::bake(&flash)),
            Role::Lamp { .. } => None,
        };
        if role == Role::Siren && pattern.is_none() {
            continue;
        }
        for place in places(lamps, record, role, to_car) {
            for layer in &layers {
                let direction = match role {
                    Role::Siren => [0.0, 0.0, 1.0],
                    Role::Lamp { .. } => place.shines,
                };
                let look = Look { position: place.at, direction, falloff, ..layer.clone() };
                let row = match (role, &pattern) {
                    (Role::Lamp { lit, .. }, _) => Row::Flare { lit, side: side_of(place.at[1]), look },
                    (Role::Siren, Some((digits, step))) => {
                        Row::Bar { siren: siren_colour(look.colour), look, pattern: digits.clone(), step: *step }
                    }
                    (Role::Siren, None) => continue,
                };
                let flares = out.iter().filter(|row| matches!(row, Row::Flare { .. })).count();
                let bars = out.len() - flares;
                let full = match row {
                    Row::Flare { .. } => flares >= MAX_FLARES,
                    Row::Bar { .. } => bars >= MAX_BARS,
                };
                if full || out.iter().any(|known| same_lamp(known, &row)) {
                    continue;
                }
                out.push(row);
            }
        }
    }
    out
}

/// Whether two rows are the same sprite of the same kind of lamp at one place.
fn same_lamp(a: &Row, b: &Row) -> bool {
    let key = |row: &Row| match row {
        Row::Flare { lit, look, .. } => (lit.to_string(), look.clone()),
        Row::Bar { siren, look, .. } => (siren.to_string(), look.clone()),
    };
    let ((kind_a, a), (kind_b, b)) = (key(a), key(b));
    kind_a == kind_b && a.sprite == b.sprite && distance(a.position, b.position) < SAME_PLACE
}

/// The colour a siren lamp is, from its sprite's tint: red, blue, or white.
fn siren_colour([r, _, b]: [f32; 3]) -> &'static str {
    match (r > b * 1.3, b > r * 1.3) {
        (true, _) => "red",
        (_, true) => "blue",
        _ => "white",
    }
}

/// The sprites of a corona: its glow layers (a flare layer when it has none), at most [`MAX_LAYERS`], each with the radius and
/// brightness at the near and far distance.
fn sprites_of(corona: &Corona, record: &CoronaRecord, sprite_of: &mut dyn FnMut(u64) -> Option<String>) -> Vec<Look> {
    let factors = RecordFactors { intensity: record.intensity, size: record.size, tint: record.tint };
    let glows = corona.glows.iter().filter(|layer| layer.kind == LayerKind::Glow);
    let mut chosen: Vec<&Layer> = glows.take(MAX_LAYERS).collect();
    if chosen.is_empty() {
        chosen = corona.flares.iter().take(1).collect();
    }
    let mut out = Vec::new();
    for layer in chosen {
        let Some(sprite) = layer.picture.and_then(&mut *sprite_of) else { continue };
        let (near, far) = (corona.seen(layer, &factors, 0.0, NEAR), corona.seen(layer, &factors, 0.0, FAR));
        let strength =
            |seen: &chameleon_effects::corona::Seen| seen.colour.iter().copied().fold(0.0, f32::max) * seen.alpha;
        let (strengths, peak) = ([strength(&near), strength(&far)], strength(&near).max(strength(&far)));
        if peak <= 1e-4 {
            continue;
        }
        let brightest = match strengths[0] >= strengths[1] {
            true => near,
            false => far,
        };
        let top = brightest.colour.iter().copied().fold(0.0, f32::max);
        let colour = brightest.colour.map(|c| (c / top).clamp(0.0, 1.0));
        let radius = |seen: &chameleon_effects::corona::Seen| {
            ((seen.size[0] * seen.size[1]).max(0.0).sqrt() / 2.0).clamp(RADIUS.0, RADIUS.1)
        };
        out.push(Look {
            sprite,
            position: [0.0; 3],
            direction: [0.0; 3],
            colour,
            alpha: [strengths[0] / peak, strengths[1] / peak],
            radius: [radius(&near), radius(&far)],
            falloff: 0.0,
        });
    }
    out
}

/// The manifest tables of the rows, starting with a newline.
pub(crate) fn manifest(rows: &[Row]) -> String {
    let mut text = String::new();
    for row in rows {
        match row {
            Row::Flare { lit, side, look } => {
                text.push_str(&format!("\n[[lamp_flare]]\nstate = \"{lit}\"\nside = \"{side}\"\n"));
                text.push_str(&look_text(look));
            }
            Row::Bar { siren, look, pattern, step } => {
                text.push_str(&format!(
                    "\n[[light_bar]]\nsiren = \"{siren}\"\npattern = \"{pattern}\"\nstep_seconds = {step:?}\n"
                ));
                text.push_str(&look_text(look));
            }
        }
    }
    text
}

fn look_text(look: &Look) -> String {
    let three = |v: [f32; 3]| format!("[{:?}, {:?}, {:?}]", v[0], v[1], v[2]);
    let two = |v: [f32; 2]| format!("[{:?}, {:?}]", v[0], v[1]);
    format!(
        "sprite = \"{}\"\nposition = {}\ndirection = {}\ncolour = {}\nalpha = {}\nradius = {}\nrange = {}\nfalloff = {:?}\n",
        look.sprite,
        three(look.position),
        three(look.direction),
        three(look.colour),
        two(look.alpha),
        two(look.radius),
        two([NEAR, FAR]),
        look.falloff
    )
}
