//! A collision spark profile: what a mod's collision spark variant looks like (`docs/modding/effect-variants-sparks-xenon.md`).
//!
//! The file is `SPARKS/<value id>/profile.toml` in the mod's `files/` folder, where `<value id>` is the id of the value the mod
//! adds to the `collision_spark_variant` option. It replaces the particle groups the game's **Original PC** spark style reads
//! from the install for a vehicle's hits and scrapes. Each channel (`[hit]`, `[scrape]`, gates in `spark_gate`) lists up to
//! eight layers; one layer is one emitter, played where the car touched the surface, in the frame of the surface (the local `z`
//! axis is the surface's normal).
//!
//! ```toml
//! format = 1
//!
//! [hit]
//! closing_speed_full = 22.08
//! intensity_floor = 0.153
//!
//! [[hit.layer]]
//! kind = "streak"              # "streak" (a quad along the velocity) or "sprite" (a picture facing the camera)
//! burst = 79.0                 # particles at once at full strength; in [[scrape.layer]] `rate`, particles a second
//! life = 0.85                  # seconds; life_variance (0 to 1) is how much of it a particle may fall short of
//! speed = 8.0                  # m/s along the normal at full strength, scaled by the strength; speed_variance, spread (degrees)
//! gravity = 9.8                # m/s2, down; drag is the share of its speed a particle loses a second for each m/s
//! stretch = 0.125              # a streak shows this many seconds of its travel
//! keys = [0.0, 0.33, 0.66, 0.99]
//! size = [0.08, 0.0184, 0.0118, 0.0052]                          # full width in metres at each key
//! colors = [[255, 104, 39, 255], [255, 104, 39, 255], [255, 112, 36, 230], [255, 125, 28, 0]]
//! brightness = [12.0, 6.0, 6.0, 5.3]                             # an overbright multiplier of the colour at each key
//! ```

use serde::Deserialize;

use super::ContentError;
use super::engine_sound::is_png_path;
use super::spark_gate::{self, HitChannel, ScrapeChannel};
use super::variant::{self, COLLISION_SPARKS};

/// The folder under a mod's `files/` that holds the variants' files.
pub const SPARKS_FOLDER: &str = COLLISION_SPARKS.folder;
/// The most layers one channel may have.
pub const MAX_SPARK_LAYERS: usize = 8;
/// The most keys along a particle's life.
pub const MAX_SPARK_KEYS: usize = 16;
/// The most particles a layer throws at once or in a second at full strength (the game's PC budget).
pub const MAX_SPARK_COUNT: f32 = 1024.0;

/// The path of the profile of variant `value` in the install's layout.
pub fn sparks_profile_path(value: &str) -> String {
    COLLISION_SPARKS.profile_path(value)
}

/// How a layer's particles are drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SparkKind {
    /// A quad along the particle's velocity, as wide as `size`, showing `stretch` seconds of its travel (a round dot with no
    /// `stretch`). It needs no picture.
    Streak,
    /// A picture (or one cell of a sheet) that faces the camera, as wide as `size`.
    Sprite,
}

/// One emitter of a channel.
#[derive(Debug, Clone, PartialEq)]
pub struct SparkLayer {
    pub kind: SparkKind,
    /// An RGBA PNG in the mod's `files/`. A sprite has one; a streak may name one, and has the game's own streak when it does not.
    pub texture: Option<String>,
    pub additive: bool,
    /// 0 for the whole picture, else 2, 4 or 8 cells per side.
    pub grid: u8,
    /// A sheet's frames a second (0 to 60), or a random cell for each particle with `random_frame` and 0.
    pub fps: u8,
    pub random_frame: bool,
    /// Particles at once at full strength (an impact), or a second at full strength (a scrape).
    pub count: f32,
    /// Seconds a particle lives at most, and the share of that it may fall short of.
    pub life: f32,
    pub life_variance: f32,
    /// Metres a second along the surface's normal at full strength, its variance (a share), and the cone's half angle in degrees.
    pub speed: f32,
    pub speed_variance: f32,
    pub spread: f32,
    /// Metres a second squared, down, and the share of its speed a particle loses each second for each m/s it goes.
    pub gravity: f32,
    pub drag: f32,
    /// The share of the normal speed kept at a bounce, and of the tangent speed lost at one (each a range). The Original PC
    /// style has no bounce and ignores them; they are kept for the profile's other readers.
    pub bounce: Option<[f32; 2]>,
    pub friction: Option<[f32; 2]>,
    /// The share of the owner's velocity a particle takes at birth, and the share of it a particle may fall short of.
    pub inherit: f32,
    pub inherit_variance: f32,
    /// Seconds of travel a streak shows; 0 is a round dot.
    pub stretch: f32,
    /// Where `size`, `colors` and `brightness` sit along the life, 0 to 1, rising: 2 to 16. Between the keys they change in
    /// straight lines.
    pub keys: Vec<f32>,
    /// Full width in metres at each key.
    pub size: Vec<f32>,
    /// Red, green, blue and alpha at each key.
    pub colors: Vec<[u8; 4]>,
    /// The overbright multiplier of the colour at each key (1 is the colour as it is).
    pub brightness: Vec<f32>,
}

/// A collision spark variant: what is thrown at an impact and at a scrape. A channel with no layer keeps the install's sparks.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SparksProfile {
    pub hit: HitChannel<SparkLayer>,
    pub scrape: ScrapeChannel<SparkLayer>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawLayer {
    kind: Option<String>,
    texture: Option<String>,
    blend: Option<String>,
    grid: Option<u8>,
    fps: Option<u8>,
    random_frame: Option<bool>,
    burst: Option<f32>,
    rate: Option<f32>,
    life: Option<f32>,
    life_variance: Option<f32>,
    speed: Option<f32>,
    speed_variance: Option<f32>,
    spread: Option<f32>,
    gravity: Option<f32>,
    drag: Option<f32>,
    bounce: Option<[f32; 2]>,
    friction: Option<[f32; 2]>,
    inherit: Option<f32>,
    inherit_variance: Option<f32>,
    stretch: Option<f32>,
    keys: Option<Vec<f32>>,
    size: Option<Vec<f32>>,
    colors: Option<Vec<[u8; 4]>>,
    brightness: Option<Vec<f32>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawHit {
    closing_speed_full: Option<f32>,
    intensity_floor: Option<f32>,
    #[serde(default)]
    layer: Vec<RawLayer>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawScrape {
    speed_start: Option<f32>,
    speed_full: Option<f32>,
    #[serde(default)]
    layer: Vec<RawLayer>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Raw {
    format: u32,
    hit: Option<RawHit>,
    scrape: Option<RawScrape>,
}

/// Parse and check a profile. A number out of range, a picture path that is not a `.PNG` path, a key the schema does not
/// know, a layer missing what it needs, a profile that plays nothing or another format number is refused with the reason.
pub fn parse(text: &str) -> Result<SparksProfile, ContentError> {
    let raw: Raw = toml::from_str(text).map_err(|error| ContentError::Parse(error.to_string()))?;
    variant::check_format(raw.format)?;
    let mut profile = SparksProfile::default();
    if let Some(hit) = raw.hit {
        let items = layers(hit.layer, true)?;
        let gate = spark_gate::hit_gate(hit.closing_speed_full, hit.intensity_floor, !items.is_empty())?;
        profile.hit = HitChannel { gate, items };
    }
    if let Some(scrape) = raw.scrape {
        let items = layers(scrape.layer, false)?;
        let gate = spark_gate::scrape_gate(scrape.speed_start, scrape.speed_full, !items.is_empty())?;
        profile.scrape = ScrapeChannel { gate, items };
    }
    if profile.hit.items.is_empty() && profile.scrape.items.is_empty() {
        return Err(ContentError::Field {
            field: "layer",
            reason: "a spark profile needs a layer in [hit] or [scrape]",
        });
    }
    Ok(profile)
}

fn layers(raw: Vec<RawLayer>, hit: bool) -> Result<Vec<SparkLayer>, ContentError> {
    if raw.len() > MAX_SPARK_LAYERS {
        return Err(ContentError::Field { field: "layer", reason: "a channel has at most 8 layers" });
    }
    raw.into_iter().map(|raw| layer(raw, hit)).collect()
}

fn within(value: Option<f32>, default: f32, low: f32, high: f32, field: &'static str) -> Result<f32, ContentError> {
    let value = value.unwrap_or(default);
    if !value.is_finite() || value < low || value > high {
        return Err(ContentError::Field { field, reason: "a number outside the range this field takes" });
    }
    Ok(value)
}

/// A pair of shares from 0 to 1 (a range).
fn share_range(value: Option<[f32; 2]>, field: &'static str) -> Result<Option<[f32; 2]>, ContentError> {
    let Some(pair) = value else { return Ok(None) };
    let ok = pair.iter().all(|share| share.is_finite() && (0.0..=1.0).contains(share)) && pair[0] <= pair[1];
    if !ok {
        return Err(ContentError::Field { field, reason: "two shares from 0 to 1, the lower first" });
    }
    Ok(Some(pair))
}

fn layer(raw: RawLayer, hit: bool) -> Result<SparkLayer, ContentError> {
    let kind = match raw.kind.as_deref() {
        None | Some("sprite") => SparkKind::Sprite,
        Some("streak") => SparkKind::Streak,
        Some(_) => return Err(ContentError::Field { field: "layer.kind", reason: "streak or sprite" }),
    };
    let additive = match raw.blend.as_deref() {
        None | Some("additive") => true,
        Some("alpha") => false,
        Some(_) => return Err(ContentError::Field { field: "layer.blend", reason: "additive or alpha" }),
    };
    let grid = raw.grid.unwrap_or(0);
    if !matches!(grid, 0 | 2 | 4 | 8) {
        return Err(ContentError::Field { field: "layer.grid", reason: "0, 2, 4 or 8" });
    }
    match raw.texture.as_deref() {
        Some(path) if !is_png_path(path) => {
            return Err(ContentError::Field {
                field: "layer.texture",
                reason: "a relative .PNG path of letters, digits, _ - . /",
            });
        }
        None if kind == SparkKind::Sprite => {
            return Err(ContentError::Field { field: "layer.texture", reason: "a sprite needs a picture" });
        }
        _ => {}
    }
    let count = match (hit, raw.burst, raw.rate) {
        (true, Some(burst), None) => burst,
        (false, None, Some(rate)) => rate,
        (true, _, _) => {
            return Err(ContentError::Field { field: "layer.burst", reason: "an impact layer has burst and no rate" });
        }
        (false, _, _) => {
            return Err(ContentError::Field { field: "layer.rate", reason: "a scrape layer has rate and no burst" });
        }
    };
    if !count.is_finite() || !(0.0..=MAX_SPARK_COUNT).contains(&count) {
        return Err(ContentError::Field { field: "layer.count", reason: "a count from 0 to 1024" });
    }
    let keys = raw.keys.unwrap_or_else(|| vec![0.0, 1.0 / 3.0, 2.0 / 3.0, 1.0]);
    let at = keys.len();
    if !(2..=MAX_SPARK_KEYS).contains(&at) {
        return Err(ContentError::Field { field: "layer.keys", reason: "2 to 16 numbers" });
    }
    let rising = keys.windows(2).all(|pair| pair[0] < pair[1]);
    if !rising || keys.iter().any(|k| !k.is_finite() || !(0.0..=1.0).contains(k)) {
        return Err(ContentError::Field { field: "layer.keys", reason: "strictly rising numbers from 0 to 1" });
    }
    let size = raw.size.unwrap_or_else(|| vec![0.05; at]);
    if size.len() != at || size.iter().any(|s| !s.is_finite() || !(0.0..=10.0).contains(s)) {
        return Err(ContentError::Field { field: "layer.size", reason: "a number from 0 to 10 metres for each key" });
    }
    let colors = raw.colors.unwrap_or_else(|| vec![[255; 4]; at]);
    if colors.len() != at {
        return Err(ContentError::Field { field: "layer.colors", reason: "a colour for each key" });
    }
    let brightness = raw.brightness.unwrap_or_else(|| vec![1.0; at]);
    if brightness.len() != at || brightness.iter().any(|b| !b.is_finite() || !(0.0..=64.0).contains(b)) {
        return Err(ContentError::Field { field: "layer.brightness", reason: "a number from 0 to 64 for each key" });
    }
    Ok(SparkLayer {
        kind,
        texture: raw.texture,
        additive,
        grid,
        fps: within(raw.fps.map(f32::from), 0.0, 0.0, 60.0, "layer.fps")? as u8,
        random_frame: raw.random_frame.unwrap_or(false),
        count,
        life: within(raw.life, 0.5, 0.01, 10.0, "layer.life")?,
        life_variance: within(raw.life_variance, 0.0, 0.0, 1.0, "layer.life_variance")?,
        speed: within(raw.speed, 0.0, 0.0, 200.0, "layer.speed")?,
        speed_variance: within(raw.speed_variance, 0.0, 0.0, 1.0, "layer.speed_variance")?,
        spread: within(raw.spread, 0.0, 0.0, 180.0, "layer.spread")?,
        gravity: within(raw.gravity, 0.0, -50.0, 50.0, "layer.gravity")?,
        drag: within(raw.drag, 0.0, 0.0, 10.0, "layer.drag")?,
        bounce: share_range(raw.bounce, "layer.bounce")?,
        friction: share_range(raw.friction, "layer.friction")?,
        inherit: within(raw.inherit, 0.0, 0.0, 1.0, "layer.inherit")?,
        inherit_variance: within(raw.inherit_variance, 0.0, 0.0, 1.0, "layer.inherit_variance")?,
        stretch: within(raw.stretch, 0.0, 0.0, 5.0, "layer.stretch")?,
        keys,
        size,
        colors,
        brightness,
    })
}

#[cfg(test)]
#[path = "spark_variant_tests.rs"]
mod tests;
