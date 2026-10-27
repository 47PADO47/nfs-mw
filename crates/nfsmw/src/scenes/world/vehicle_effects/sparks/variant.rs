//! The restored spark variant a mod gives (`XENON/<value>/profile.toml`, `docs/modding/effect-variants-sparks-xenon.md`): the
//! numbers of the two line sparks the **Restored (Experimental)** style throws at an impact and at a scrape.
//!
//! The install's triggers stay: a contact must close or slide, and the surface must have a restored spark of its own
//! (`CollisionEffects`), for anything to be thrown. With a variant chosen, a channel the profile has sparks for is played
//! from the profile instead of the install: the strength comes from the profile's gate (`[hit]`: the closing speed, `[scrape]`:
//! the sliding speed), and the strength scales the count of an impact and its speed; a scrape scales the rate. A channel the
//! profile has no spark for keeps the install's, exactly as it was. The wind contrail and the view streaks are not here: the
//! 2012 game has none, so the variant is the sparks only.

use std::sync::Arc;

use blackbox_modding::content::{XenonProfile, XenonSpark};
use glam::{Quat, Vec3};
use nfsmw_data::vehicle_effects::{EmitterStyle, SparkLink};

use super::super::super::{drive::VisualContact, space};
use super::super::particle::Particle;
use super::{Sparks, intensity};
use crate::settings::XenonEffects;

/// The most particles one population throws in a step.
const MOST_PER_STEP: usize = 256;
/// The elasticity (over 255) the restored style gives its two populations when the profile does not say.
const OWN_ELASTICITY: [f32; 2] = [160.0, 120.0];

#[cfg(test)]
#[path = "variant_tests.rs"]
mod tests;

/// What a channel of the profile plays at one contact: its sparks as the restored emitters, and the strength.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::scenes::world::vehicle_effects) struct Played {
    pub styles: [Option<EmitterStyle>; 2],
    /// The share (0 to 1) of the speed a spark keeps at a bounce, where the profile says.
    pub elasticity: [Option<f32>; 2],
    pub strength: f32,
}

impl Played {
    /// The link the rest of the emitting code (the contact flash) reads: the profile's sparks, the whole velocity of the car.
    fn link(&self) -> SparkLink {
        SparkLink { min: 0.0, max: 1.0, styles: self.styles, inherit_velocity: 1.0 }
    }
}

/// The restored emitter a spark of the profile stands for (both keep the restoration's units).
fn style_of(spark: &XenonSpark) -> EmitterStyle {
    EmitterStyle {
        color: spark.color,
        life: spark.life,
        life_variance: spark.life_variance,
        count: spark.count,
        count_variance: spark.count_variance,
        center: spark.center,
        extent: spark.extent,
        velocity: spark.velocity,
        variation: spark.variation,
        inherit: spark.inherit,
        gravity: spark.gravity,
        gravity_delta: spark.gravity_delta,
        length: spark.length,
        length_delta: spark.length_delta,
        height: spark.height,
    }
}

fn played(sparks: &[XenonSpark], strength: f32) -> Played {
    let mut played = Played { styles: [None; 2], elasticity: [None; 2], strength };
    for (slot, spark) in sparks.iter().take(2).enumerate() {
        played.styles[slot] = Some(style_of(spark));
        played.elasticity[slot] = spark.elasticity.map(|value| value / 255.0);
    }
    played
}

/// What an impact at `closing` m/s plays, or `None` when the profile has no impact spark.
pub(super) fn hit_played(profile: &XenonProfile, closing: f32) -> Option<Played> {
    let strength = profile.hit.strength(closing)?;
    Some(played(&profile.hit.items, strength))
}

/// What a scrape at `sliding` m/s plays, or `None` when the profile has no scrape spark.
pub(super) fn scrape_played(profile: &XenonProfile, sliding: f32) -> Option<Played> {
    let strength = profile.scrape.strength(sliding)?;
    Some(played(&profile.scrape.items, strength))
}

/// The link, strength and variant play of an impact whose surface has `link` in the install. Without a variant spark for impacts
/// they are the install's: its strength from the contact's impulse.
pub(super) fn hit(
    profile: Option<&XenonProfile>,
    link: SparkLink,
    closing: f32,
    delta_v: f32,
) -> (SparkLink, f32, Option<Played>) {
    let Some(played) = profile.and_then(|profile| hit_played(profile, closing)) else {
        return (link, intensity(link, delta_v, false), None);
    };
    (played.link(), played.strength, Some(played))
}

/// The same for a scrape at the sliding speed `sliding`.
pub(super) fn scrape(
    profile: Option<&XenonProfile>,
    link: SparkLink,
    sliding: f32,
) -> (SparkLink, f32, Option<Played>) {
    let Some(played) = profile.and_then(|profile| scrape_played(profile, sliding)) else {
        return (link, intensity(link, sliding, true), None);
    };
    (played.link(), played.strength, Some(played))
}

/// How many particles a population throws: an impact throws its count at once, a scrape its count a second for `dt` seconds.
/// `carry` keeps the part of a particle a scrape owes, so the rate is right at any step.
fn amount(style: &EmitterStyle, strength: f32, boost: f32, hit: bool, dt: f32, carry: &mut f32) -> usize {
    let spread = (1.0 - style.count_variance * 100.0).max(0.0);
    let wanted = style.count * strength * boost * spread;
    if hit {
        return (wanted.round().max(0.0) as usize).min(MOST_PER_STEP);
    }
    let total = *carry + wanted * dt.max(0.0);
    let whole = total.floor().max(0.0);
    *carry = total - whole;
    (whole as usize).min(MOST_PER_STEP)
}

impl Sparks {
    /// The variant in effect, or `None` for the install's sparks. A new profile starts without the debt of the old.
    pub fn set_variant(&mut self, profile: Option<Arc<XenonProfile>>) {
        let same = match (&self.variant, &profile) {
            (Some(have), Some(want)) => Arc::ptr_eq(have, want),
            (None, None) => true,
            _ => false,
        };
        if same {
            return;
        }
        self.variant = profile;
        self.variant_carry = [0.0; 2];
    }

    /// Throw the sparks of a variant's channel from the contact `c`: an impact once, a scrape for this step of `dt` seconds.
    pub(super) fn spawn_variant(
        &mut self,
        c: &VisualContact,
        played: &Played,
        hit: bool,
        tuning: &XenonEffects,
        dt: f32,
    ) {
        let normal = space::to_render(c.normal.to_array());
        let origin = space::to_render(c.point.to_array());
        // The profile's local frame has `z` along the surface normal for both channels.
        let rotation = Quat::from_rotation_arc(Vec3::Z, normal);
        let boost = tuning.spark_intensity.max(1.0);
        for (index, style) in played.styles.into_iter().enumerate() {
            let Some(mut style) = style else { continue };
            let mut carry = self.variant_carry[index];
            let births = amount(&style, played.strength, boost, hit, dt, &mut carry);
            self.variant_carry[index] = carry;
            if hit {
                // An impact throws its sparks as fast as it is strong; a scrape changes the rate only.
                style.velocity = style.velocity.map(|speed| speed * played.strength);
            }
            let elasticity = played.elasticity[index].unwrap_or(OWN_ELASTICITY[index] / 255.0);
            for _ in 0..births {
                let seed = self.emitted;
                let mut p = Particle::spawn(style, origin, rotation, self.owner_velocity, seed, tuning.spark_intensity);
                p.origin += normal * (0.02 - (p.origin - origin).dot(normal)).max(0.0);
                p.previous = p.origin;
                p.elasticity = elasticity;
                while self.particles.len() >= tuning.max_sparks {
                    self.particles.pop_front();
                }
                self.particles.push_back(p);
                self.emitted += 1;
            }
        }
    }
}
