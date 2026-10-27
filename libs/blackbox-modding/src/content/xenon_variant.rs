//! A restored spark profile: what a mod's variant of the **Restored (Experimental)** collision sparks looks like
//! (`docs/modding/effect-variants-sparks-xenon.md`).
//!
//! The file is `XENON/<value id>/profile.toml` in the mod's `files/` folder, where `<value id>` is the id of the value the mod
//! adds to the `xenon_spark_variant` option. It replaces the numbers of the two line sparks the restored style throws at an
//! impact and at a scrape, the way a nitrous profile replaces the numbers of the flames. One `[[hit.spark]]` is one spark
//! population (up to two a channel); its keys are the fields of the restoration's emitter. There is no trail and no streak:
//! the restored style's wind contrail and view streaks have no variant.
//!
//! ```toml
//! format = 1
//!
//! [hit]                          # gates: see `spark_gate`
//! closing_speed_full = 22.08
//! intensity_floor = 0.153
//!
//! [[hit.spark]]
//! color = [1.0, 0.407, 0.153, 1.0]   # RGBA, 0 to 1
//! life = 0.55                    # seconds; the sparks have one fixed life: life * (1 - life_variance)
//! count = 79.0                   # particles at once at full strength; in [[scrape.spark]] particles a second
//! center = [0.0, 0.0, 0.0]       # the birth volume, local frame (z along the surface normal), metres
//! extent = [0.5, 0.5, 0.0]
//! velocity = [0.53, 0.53, 6.0]   # local start velocity, m/s, scaled by the strength
//! variation = [1.0, 1.0, 0.333]  # each component times 1 + variation * (2u - 1)
//! inherit = [0.55, 0.55, 0.0]    # share of the incoming world velocity
//! gravity = -4.9                 # the coefficient of the parabola (half the acceleration)
//! length = 255.0                 # 0 to 255: time offset of the segment's tail, over 2048 seconds
//! height = 59.0                  # 0 to 255: full width, over 2048 metres
//! elasticity = 217.0             # 0 to 255, over 255: the share of the speed kept at a swept bounce
//! ```

use serde::Deserialize;

use super::ContentError;
use super::spark_gate::{self, HitChannel, ScrapeChannel};
use super::variant::{self, XENON_SPARKS};

/// The folder under a mod's `files/` that holds the variants' files.
pub const XENON_FOLDER: &str = XENON_SPARKS.folder;
/// The most sparks a channel may have (the restored style has two line emitters).
pub const MAX_XENON_SPARKS: usize = 2;

/// The path of the profile of variant `value` in the install's layout.
pub fn xenon_profile_path(value: &str) -> String {
    XENON_SPARKS.profile_path(value)
}

/// One population of line sparks: the fields of the restoration's emitter, in its units.
#[derive(Debug, Clone, PartialEq)]
pub struct XenonSpark {
    pub color: [f32; 4],
    pub life: f32,
    pub life_variance: f32,
    /// Particles at once at full strength (an impact), or a second at full strength (a scrape).
    pub count: f32,
    pub count_variance: f32,
    pub center: [f32; 3],
    pub extent: [f32; 3],
    pub velocity: [f32; 3],
    pub variation: [f32; 3],
    pub inherit: [f32; 3],
    pub gravity: f32,
    pub gravity_delta: f32,
    pub length: f32,
    pub length_delta: f32,
    pub height: f32,
    /// The share (over 255) of the speed a spark keeps at a swept bounce, 0 to 255. `None` keeps the style's own (160 and 120).
    pub elasticity: Option<f32>,
}

/// A restored spark variant. A channel with no spark keeps the install's.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct XenonProfile {
    pub hit: HitChannel<XenonSpark>,
    pub scrape: ScrapeChannel<XenonSpark>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawSpark {
    color: [f32; 4],
    life: f32,
    life_variance: Option<f32>,
    count: f32,
    count_variance: Option<f32>,
    center: Option<[f32; 3]>,
    extent: Option<[f32; 3]>,
    velocity: Option<[f32; 3]>,
    variation: Option<[f32; 3]>,
    inherit: Option<[f32; 3]>,
    gravity: Option<f32>,
    gravity_delta: Option<f32>,
    length: Option<f32>,
    length_delta: Option<f32>,
    height: Option<f32>,
    elasticity: Option<f32>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawHit {
    closing_speed_full: Option<f32>,
    intensity_floor: Option<f32>,
    #[serde(default)]
    spark: Vec<RawSpark>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawScrape {
    speed_start: Option<f32>,
    speed_full: Option<f32>,
    #[serde(default)]
    spark: Vec<RawSpark>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Raw {
    format: u32,
    hit: Option<RawHit>,
    scrape: Option<RawScrape>,
}

/// Parse and check a profile. A number out of range (the ranges of the restoration's emitter), a key the schema does not know,
/// a channel with sparks and no gate, a profile that plays nothing or another format number is refused with the reason.
pub fn parse(text: &str) -> Result<XenonProfile, ContentError> {
    let raw: Raw = toml::from_str(text).map_err(|error| ContentError::Parse(error.to_string()))?;
    variant::check_format(raw.format)?;
    let mut profile = XenonProfile::default();
    if let Some(hit) = raw.hit {
        let items = sparks(hit.spark)?;
        let gate = spark_gate::hit_gate(hit.closing_speed_full, hit.intensity_floor, !items.is_empty())?;
        profile.hit = HitChannel { gate, items };
    }
    if let Some(scrape) = raw.scrape {
        let items = sparks(scrape.spark)?;
        let gate = spark_gate::scrape_gate(scrape.speed_start, scrape.speed_full, !items.is_empty())?;
        profile.scrape = ScrapeChannel { gate, items };
    }
    if profile.hit.items.is_empty() && profile.scrape.items.is_empty() {
        return Err(ContentError::Field { field: "spark", reason: "a profile needs a spark in [hit] or [scrape]" });
    }
    Ok(profile)
}

fn sparks(raw: Vec<RawSpark>) -> Result<Vec<XenonSpark>, ContentError> {
    if raw.len() > MAX_XENON_SPARKS {
        return Err(ContentError::Field { field: "spark", reason: "a channel has at most 2 sparks" });
    }
    raw.into_iter().map(spark).collect()
}

fn within(value: f32, low: f32, high: f32, field: &'static str) -> Result<f32, ContentError> {
    if !value.is_finite() || value < low || value > high {
        return Err(ContentError::Field { field, reason: "a number outside the range this field takes" });
    }
    Ok(value)
}

/// Three numbers each within `low` to `high`.
fn within3(value: [f32; 3], low: f32, high: f32, field: &'static str) -> Result<[f32; 3], ContentError> {
    for number in value {
        within(number, low, high, field)?;
    }
    Ok(value)
}

fn spark(raw: RawSpark) -> Result<XenonSpark, ContentError> {
    let mut color = raw.color;
    for channel in &mut color {
        *channel = within(*channel, 0.0, 1.0, "spark.color")?;
    }
    let life = within(raw.life, 0.0, 10.0, "spark.life")?;
    if life == 0.0 {
        return Err(ContentError::Field { field: "spark.life", reason: "above 0 seconds" });
    }
    // The restoration keeps 0 <= variance < 1 for a life, so a spark always lives a while.
    let life_variance = within(raw.life_variance.unwrap_or(0.0), 0.0, 0.999, "spark.life_variance")?;
    Ok(XenonSpark {
        color,
        life,
        life_variance,
        count: within(raw.count, 0.0, 256.0, "spark.count")?,
        count_variance: within(raw.count_variance.unwrap_or(0.0), 0.0, 0.01, "spark.count_variance")?,
        center: within3(raw.center.unwrap_or([0.0; 3]), -1000.0, 1000.0, "spark.center")?,
        extent: within3(raw.extent.unwrap_or([0.0; 3]), 0.0, 1000.0, "spark.extent")?,
        velocity: within3(raw.velocity.unwrap_or([0.0; 3]), -1000.0, 1000.0, "spark.velocity")?,
        variation: within3(raw.variation.unwrap_or([0.0; 3]), 0.0, 1000.0, "spark.variation")?,
        inherit: within3(raw.inherit.unwrap_or([0.0; 3]), -1000.0, 1000.0, "spark.inherit")?,
        gravity: within(raw.gravity.unwrap_or(0.0), -1000.0, 1000.0, "spark.gravity")?,
        gravity_delta: within(raw.gravity_delta.unwrap_or(0.0), -1000.0, 1000.0, "spark.gravity_delta")?,
        length: within(raw.length.unwrap_or(0.0), 0.0, 255.0, "spark.length")?,
        length_delta: within(raw.length_delta.unwrap_or(0.0), 0.0, 255.0, "spark.length_delta")?,
        height: within(raw.height.unwrap_or(0.0), 0.0, 255.0, "spark.height")?,
        elasticity: raw.elasticity.map(|value| within(value, 0.0, 255.0, "spark.elasticity")).transpose()?,
    })
}

#[cfg(test)]
#[path = "xenon_variant_tests.rs"]
mod tests;
