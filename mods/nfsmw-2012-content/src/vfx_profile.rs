//! The `[[effect]]` entries and the pictures of the 2012 nitrous variant, made from the emitters of the three 2012 effects
//! (`vfx`) with the numbers the effects themselves hold (`chameleon-effects`, docs/formats/chameleon-effects.md):
//!
//! | The profile's | From the emitter |
//! |---|---|
//! | `rate`, `rate_per_speed` | the spawner's rate at rest, and for each m/s of the car's speed |
//! | `life`, `life_variance` | the longest lifetime, and the share of it the shortest falls short by |
//! | `duration` | the end of the spawner's time window (a burn lasts as long as the nitrous does) |
//! | `size`, `keys` | twice the starting radius's mean times the radius curve, at four points of the life |
//! | `colors` | the colour gradient at those points with its brightness multiplied in, and the alpha curve |
//! | `lift`, `damping` | the motion's upward acceleration, and the share of the velocity it keeps losing at every step |
//! | `speed`, `spread` | the spawner's velocity scale, thrown in random directions at a random share of it |
//! | `inherit`, `inherit_variance` | the velocity inheritance range; an emitter whose particles follow it takes all of it |
//! | `angle_range` | the starting rotation range |
//!
//! What the rewrite adds against the 2012 screenshots, which the decoded numbers do not give (each is a constant below, none is
//! read from the install): the flames and the glow leave the pipe along its axis (a sideways pipe flames sideways), the glow is
//! kept compact and has a small bright core (the 2012 game blooms it), and the glow spawns steadily at rest.
//!
//! What is not carried over, because it is not decoded or the host cannot play it: the wind and the turbulence of the motion,
//! the spawn offset ("shell"), the random scale channels, the spin, the sprite's draw scale (a sprite is taken to be twice its radius
//! wide), how the flipbook picks its frame (it plays once over the life), the blend (decided here from the brightness and the
//! role) and the lights. The refraction sprite (heat haze) is not a sprite the host can draw and is left out.

use chameleon_effects::decode::Gradient;
use chameleon_effects::ops::RENDER_SPRITE_UV_DISTORTION;

use crate::vfx::{Emitter, Picture};

/// The files and the text a variant's effects need.
#[derive(Debug, Clone, PartialEq)]
pub struct EffectSet {
    /// File name and picture, written under `files/NITROUS/MW2012/`.
    pub pictures: Vec<(String, Picture)>,
    /// The `[[effect]]` tables, and whether there are burn and pop effects (so the game's own flames can be replaced).
    pub toml: String,
    pub has_burn: bool,
    pub has_pop: bool,
}

/// The folder the pictures are named in.
const FOLDER: &str = "NITROUS/MW2012";
/// The cells per side of the flame sheet (25 frames), seen on the picture.
const FLAME_GRID: u8 = 5;
/// A flipbook picture at least this wide is the flame sheet; a smaller one is the dust.
const FLAME_WIDTH: u32 = 512;
/// A window that ends later than this is "for as long as the effect is on".
const OPEN_WINDOW: f32 = 1.0e5;
/// The width of a sprite in radii (a sprite's radius is its half width).
const WIDTH_PER_RADIUS: f32 = 2.0;
/// The spawn rate's second term multiplies the length of a vector input of the effect (`Velocity` or the movement since the last
/// frame: the cone block does not say which). Read as the car's speed, the glow's 1 particle a metre gives a dense cloud over the
/// whole car at 130 km/h that the screenshots of the game do not show; read as the movement of one frame it is a 60th of that.
/// **Chosen against the screenshots, not decoded.**
const MOVEMENT_FRAMES_PER_SECOND: f32 = 60.0;
/// Whether the glow is left behind the car as the effect file says (see `Entry::of`). Off: it rides with the car.
const KEEP_TRAIL: bool = false;
/// What the sprites are drawn at, as a factor on the width the radius gives, by the index of the parameter that scales the
/// spawn rate (a glow `P[0]`, a flame `P[1]`). **Calibrated against two screenshots of the 2012 game, not decoded**: the
/// effect's parameter array and its sprite draw scale are not read, and with a factor of 1 the glow is twice as wide as the
/// screenshots show it and the flames a third too small.
fn draw_scale(rate_param: u8) -> f32 {
    match rate_param {
        0 => 1.0,
        _ => 2.0,
    }
}
/// The widest a burning glow is drawn, metres: the width the effect gives it at the peak of its alpha (0.7 m, from 0.3 m at birth).
/// The effect grows it to 5.6 m by the end of its life, which in the 2012 game is behind the car (the glow is left in the world
/// there, fading); riding with the car it would be a blue fog round the whole of it, where the screenshots show a small glow in
/// each pipe. **Read for the peak, capped there by choice.**
const GLOW_MAX_WIDTH: f32 = 0.7;
/// The least spawn rate of a burning glow, a second: the data's own rate at rest (5) is a flicker once the glow stays small.
const GLOW_MIN_RATE: f32 = 30.0;
/// How the sprites leave the pipe when the effect gives them no speed of their own: metres a second along the pipe's axis, the
/// cone's width in degrees, and how much of the speed is random. A flame jets out (the pop and the start burst), the glow
/// streams out in a short plume. The effect files put these sprites at the pipe's origin, which buries them in the body of a
/// car whose pipe points sideways and glues them to the pipe's end; the screenshots show fire coming out of the pipe.
/// **Chosen against the screenshots, not decoded.**
/// The glow's plume is short: it lives 0.25 to 0.5 s, so 1 m/s puts the last of it half a metre from the pipe, the size the
/// glow has in the screenshots (a small blue glow at each pipe, not a jet).
const FLAME_JET: (f32, f32, f32) = (4.0, 14.0, 0.5);
const GLOW_PLUME: (f32, f32, f32) = (1.0, 10.0, 0.4);
/// The core of the glow, the bright centre the 2012 game's bloom makes of it: its width in metres at the key points, the share
/// of the way to white its colour is lightened by, its alpha at the two middle keys, and its life in seconds at most.
/// **Chosen against the screenshots, not decoded.**
const CORE_WIDTH: [f32; 4] = [0.2, 0.32, 0.36, 0.3];
const CORE_WHITE: f32 = 0.75;
const CORE_ALPHA: [u8; 2] = [235, 190];
const CORE_LIFE: f32 = 0.25;
/// The emitters whose spawn rate is scaled by a parameter of this index or above are the smoke puffs of the start and the pop;
/// by the order of the effect's inputs (`Transform`, `Velocity`, `ColourOverride`, `PreCrashVelocity`, `Intensity`,
/// `ImpactClosingVelocity`, ...) those are the crash and impact inputs, which are zero when driving. **Inferred, not read**:
/// the effect parameter array is not decoded; the references (a start burst of the 2012 game) show no puffs.
const CRASH_PARAMS: u8 = 3;
/// The most keys one effect is given.
const MAX_KEYS: usize = 12;
/// The age fraction the curves are read at for the end of a life.
const LAST_MOMENT: f32 = 0.9999;
/// Degrees in one unit of the starting rotation range.
const DEGREES_PER_ROTATION_UNIT: f32 = 360.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Moment {
    Start,
    Burn,
    Pop,
}

impl Moment {
    fn name(self) -> &'static str {
        match self {
            Moment::Start => "start",
            Moment::Burn => "burn",
            Moment::Pop => "pop",
        }
    }
}

/// The effects of the three moments, from the emitters of the 2012 effects. `None` when no emitter gives anything to play.
pub fn build(start: &[Emitter], burn: &[Emitter], pop: &[Emitter]) -> Option<EffectSet> {
    let mut set = EffectSet { pictures: Vec::new(), toml: String::new(), has_burn: false, has_pop: false };
    for (moment, emitters) in [(Moment::Start, start), (Moment::Burn, burn), (Moment::Pop, pop)] {
        let sibling = longest_window(emitters);
        for emitter in emitters {
            let Some(entry) = Entry::of(moment, emitter, sibling) else { continue };
            let file = add_picture(&mut set, moment, emitter);
            set.toml.push_str(&entry.toml(&file));
            if moment == Moment::Burn {
                set.toml.push_str(&entry.core().toml(&file));
            }
            set.has_burn |= moment == Moment::Burn;
            set.has_pop |= moment == Moment::Pop;
        }
    }
    (!set.toml.is_empty()).then_some(set)
}

/// The longest finite spawner window of the effect's emitters, 0 when there is none.
fn longest_window(emitters: &[Emitter]) -> f32 {
    let ends = emitters.iter().filter_map(|e| e.fx.cone()).map(|cone| cone.window[1]);
    ends.filter(|end| *end < OPEN_WINDOW).fold(0.0, f32::max)
}

/// The file name of the emitter's picture, adding the picture once.
fn add_picture(set: &mut EffectSet, moment: Moment, emitter: &Emitter) -> String {
    let base = match (emitter.flipbook, emitter.picture.width >= FLAME_WIDTH, moment) {
        (true, true, _) => "FLAME",
        (true, false, _) => "DUST",
        (false, _, Moment::Burn) => "GLOW",
        (false, _, _) => "SMOKE",
    };
    let mut n = 1;
    loop {
        let name = if n == 1 { format!("{base}.PNG") } else { format!("{base}{n}.PNG") };
        match set.pictures.iter().find(|(known, _)| *known == name) {
            Some((_, same)) if *same == emitter.picture => return name,
            Some(_) => n += 1,
            None => {
                set.pictures.push((name.clone(), emitter.picture.clone()));
                return name;
            }
        }
    }
}

/// One `[[effect]]`, as numbers.
#[derive(Debug, Clone, PartialEq)]
struct Entry {
    on: Moment,
    additive: bool,
    grid: u8,
    fps: u8,
    rate: f32,
    rate_per_speed: f32,
    life: f32,
    life_variance: f32,
    speed: f32,
    speed_variance: f32,
    spread: f32,
    lift: f32,
    damping: f32,
    inherit: f32,
    inherit_variance: f32,
    angle_range: f32,
    keys: Vec<f32>,
    size: Vec<f32>,
    colors: Vec<[u8; 4]>,
    spin: f32,
    duration: f32,
}

impl Entry {
    /// The entry for one emitter, or `None` when it lacks the lifetime, spawner, radius or colour that the numbers come from,
    /// or is a refraction sprite.
    fn of(on: Moment, emitter: &Emitter, sibling_window: f32) -> Option<Entry> {
        let fx = &emitter.fx;
        if fx.render_op() == Some(RENDER_SPRITE_UV_DISTORTION) {
            return None;
        }
        let (life, cone, gradient) = (fx.lifetime()?, fx.cone()?, fx.gradient()?);
        if cone.params[0] >= CRASH_PARAMS {
            return None;
        }
        let (radius, curve) = (fx.radius()?, fx.radius_curve()?);
        let mean = (radius.min + radius.max) * 0.5;
        let width = |t: f32| WIDTH_PER_RADIUS * draw_scale(cone.params[0]) * mean * curve.at(t);
        let mut knots = gradient.alpha_x.to_vec();
        knots.push(curve.peak);
        knots.extend(gradient.weight_breakpoints());
        let reference = knots.iter().map(|k| width(*k)).fold(f32::MIN_POSITIVE, f32::max);
        let keys = choose_keys(&knots, MAX_KEYS, |t| {
            let [r, g, b, a] = colour_at(&gradient, t).map(|v| f32::from(v) / 255.0);
            [a, (width(t) / reference).clamp(0.0, 1.0), r, g, b]
        });
        let brightness = keys.iter().any(|k| gradient.colour_at(k.min(LAST_MOMENT))[3] > 1.0);
        let window = cone.window[1];
        let duration = match (on, window < OPEN_WINDOW) {
            (Moment::Burn, _) => 0.0,
            (_, true) => window,
            (_, false) if sibling_window > 0.0 => sibling_window,
            // No other spawner ends: one generation of particles, the effect is over when the first ones are dead.
            (_, false) => life.max,
        };
        let longest = life.max.max(0.01);
        let top = cone.velocity.iter().copied().fold(0.0, f32::max);
        // Particles that follow their emitter are put into the world from its place every frame: they ride with the car, which
        // is to say they take all of its velocity; the others take the share the effect names, once, at birth, and the motion's
        // per-step drag then leaves them where they are. The glow is of the second kind in the effect file, and would be a trail
        // of large soft sprites behind the car; the screenshots of the 2012 game show it at the pipes and no trail, so unless
        // `KEEP_TRAIL` every sprite rides with the car.
        let follows = fx.follows_emitter() || !KEEP_TRAIL;
        let inherit = fx.inherit().filter(|_| !follows);
        let share = if follows { 1.0 } else { inherit.map_or(0.0, |i| i.max).clamp(0.0, 1.0) };
        let flame = emitter.flipbook && emitter.picture.width >= FLAME_WIDTH;
        let (speed, spread, speed_variance) = match (top > 0.0, on, flame) {
            (true, ..) => (top.min(50.0), 180.0, 1.0),
            (false, Moment::Burn, _) => GLOW_PLUME,
            (false, _, true) => FLAME_JET,
            (false, _, false) => (0.0, 0.0, 0.0),
        };
        let widest = if on == Moment::Burn { GLOW_MAX_WIDTH } else { 20.0 };
        let min_rate = if on == Moment::Burn { GLOW_MIN_RATE } else { 0.0 };
        Some(Entry {
            on,
            additive: on == Moment::Burn || emitter.flipbook || brightness,
            grid: if flame { FLAME_GRID } else { 0 },
            // The flipbook plays once over a particle's life.
            fps: (f32::from(FLAME_GRID).powi(2) / longest).round().clamp(1.0, 255.0) as u8,
            rate: cone.base_rate.clamp(min_rate, 500.0),
            rate_per_speed: (cone.rate_per_speed / MOVEMENT_FRAMES_PER_SECOND).clamp(0.0, 100.0),
            life: longest.min(10.0),
            life_variance: ((longest - life.min) / longest).clamp(0.0, 1.0),
            speed,
            speed_variance,
            spread,
            lift: fx.motion().map_or(0.0, |m| m.lift).clamp(-50.0, 50.0),
            // A following particle's own velocity is the emitter's, which the motion's damping does not touch.
            damping: if follows { 0.0 } else { fx.motion().map_or(0.0, |m| m.drag).clamp(0.0, 1.0) },
            inherit: share,
            inherit_variance: match (inherit, share > 0.0) {
                (Some(i), true) => ((share - i.min) / share).clamp(0.0, 1.0),
                _ => 0.0,
            },
            angle_range: fx.rotation().map_or(0.0, |r| ((r.max - r.min).abs() * DEGREES_PER_ROTATION_UNIT).min(360.0)),
            size: keys.iter().map(|key| width(*key).clamp(0.0, widest)).collect(),
            colors: keys.iter().map(|key| colour_at(&gradient, *key)).collect(),
            spin: fx.rotation().map_or(0.0, |r| (r.spin.abs() * DEGREES_PER_ROTATION_UNIT).min(3600.0)),
            keys,
            duration: duration.clamp(0.0, 10.0),
        })
    }

    /// The bright centre of a burning glow: the same picture, a small sprite of the glow's own colour lightened towards white.
    fn core(&self) -> Entry {
        let peak = self.colors.iter().max_by_key(|c| c[3]).copied().unwrap_or([255; 4]);
        let lighten = |value: u8| (f32::from(value) + (255.0 - f32::from(value)) * CORE_WHITE).round() as u8;
        let tint = [lighten(peak[0]), lighten(peak[1]), lighten(peak[2])];
        let at = |alpha: u8| [tint[0], tint[1], tint[2], alpha];
        Entry {
            rate_per_speed: 0.0,
            life: self.life.min(CORE_LIFE),
            life_variance: 0.3,
            speed: 0.0,
            speed_variance: 0.0,
            spread: 0.0,
            lift: 0.0,
            damping: 0.0,
            inherit: 1.0,
            inherit_variance: 0.0,
            keys: vec![0.0, 0.1, 0.5, 1.0],
            size: CORE_WIDTH.to_vec(),
            colors: vec![at(0), at(CORE_ALPHA[0]), at(CORE_ALPHA[1]), at(0)],
            spin: self.spin / 2.0,
            ..self.clone()
        }
    }

    fn toml(&self, texture: &str) -> String {
        let blend = if self.additive { "additive" } else { "alpha" };
        let grid = if self.grid > 0 { format!("grid = {}\nfps = {}\n", self.grid, self.fps) } else { String::new() };
        let duration = if self.duration > 0.0 { format!("duration = {:?}\n", self.duration) } else { String::new() };
        let list = |values: &[f32]| values.iter().map(|v| format!("{v:.4}")).collect::<Vec<_>>().join(", ");
        let colors: Vec<String> =
            self.colors.iter().map(|c| format!("[{}, {}, {}, {}]", c[0], c[1], c[2], c[3])).collect();
        let colors = colors.join(", ");
        format!(
            "\n[[effect]]\non = \"{}\"\ntexture = \"{FOLDER}/{texture}\"\nblend = \"{blend}\"\n{grid}rate = {:?}\n\
             rate_per_speed = {:?}\nlife = {:?}\nlife_variance = {:?}\nspeed = {:?}\nspeed_variance = {:?}\nspread = {:?}\n\
             lift = {:?}\ndamping = {:?}\ninherit = {:?}\ninherit_variance = {:?}\nangle_range = {:?}\nspin = {:?}\nkeys = [{}]\nsize = [{}]\n\
             colors = [{colors}]\n{duration}",
            self.on.name(),
            self.rate,
            self.rate_per_speed,
            self.life,
            self.life_variance,
            self.speed,
            self.speed_variance,
            self.spread,
            self.lift,
            self.damping,
            self.inherit,
            self.inherit_variance,
            self.angle_range,
            self.spin,
            list(&self.keys),
            list(&self.size),
        )
    }
}

/// The colour at age fraction `t`: red, green and blue times the brightness, clamped to a byte, and the alpha curve's value. A
/// particle is dead when its age fraction reaches 1 and the curves start over there, so the end is sampled just before it.
fn colour_at(gradient: &Gradient, t: f32) -> [u8; 4] {
    let t = t.min(LAST_MOMENT);
    let [r, g, b, bright] = gradient.colour_at(t);
    let byte = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    [byte(r * bright), byte(g * bright), byte(b * bright), byte(gradient.alpha_at(t))]
}

/// The points along the life to sample the curves at: every knot of the curves, and 0 and 1. While there are more than
/// `most`, the interior one that the line between its neighbours misses least (over everything `sample` returns) is dropped.
fn choose_keys(knots: &[f32], most: usize, sample: impl Fn(f32) -> [f32; 5]) -> Vec<f32> {
    let mut points: Vec<f32> = knots.iter().map(|k| k.clamp(0.0, 1.0)).chain([0.0, 1.0]).collect();
    points.sort_by(f32::total_cmp);
    points.dedup_by(|a, b| (*a - *b).abs() < 1.0e-3);
    let miss = |points: &[f32], i: usize| {
        let (a, m, b) = (sample(points[i - 1]), sample(points[i]), sample(points[i + 1]));
        let u = (points[i] - points[i - 1]) / (points[i + 1] - points[i - 1]);
        (0..5).map(|k| (m[k] - (a[k] + (b[k] - a[k]) * u)).abs()).fold(0.0, f32::max)
    };
    while points.len() > most.max(2) {
        let drop = (1..points.len() - 1).min_by(|a, b| miss(&points, *a).total_cmp(&miss(&points, *b))).unwrap_or(1);
        points.remove(drop);
    }
    points
}

#[cfg(test)]
mod tests;
