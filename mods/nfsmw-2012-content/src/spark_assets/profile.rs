//! The text of the two profiles. Every key maps to a parameter the host's collision particles already have
//! (`docs/modding/effect-variants-sparks-xenon.md` has the table); the conversions are:
//!
//! | Profile | From the 2012 emitter |
//! |---|---|
//! | `life`, `life_variance` | longest life; `1 - shortest / longest` (the host draws a life between `value * (1 - variance)` and `value`) |
//! | `speed`, `speed_variance`, `spread` | longest speed; the same share; the largest angle from the surface normal |
//! | `burst`, `rate` | density per square metre times the area of half a disc of the patch radius, `pi r^2 / 2` (**nominal**: the 2012 game counts the triangles of the real mesh) |
//! | `gravity` | the bounce block's gravity, metres a second squared, down |
//! | `drag` | the 2012 loss per second `d` as the host's speed-proportional drag: `-ln(1 - d) / mean speed` |
//! | `colors`, `brightness`, `size`, `keys` | the gradient, its overbright factor and twice the radius times the radius curve at the four ages `KEYS` |
//! | `stretch` | the seconds of travel the segment shows |
//! | Xenon `velocity`, `variation` | local frame, z along the surface normal: mean speed up, sideways `mean * tan(spread / 2)` |
//! | Xenon `length`, `height` | `stretch * 2048`, `mean width * 2048`, both at most 255 |
//! | Xenon `elasticity` | the mean restitution times 255 |

use std::fmt::Write as _;

use super::{Channel, Gate, KEYS, Kind, Set, Spark};

/// The most emitters a profile lists in a channel (the host's original-style table has eight slots).
const MAX_EMITTERS: usize = 8;
/// The restored spark style has two line emitters in a channel.
const MAX_LINES: usize = 2;
/// The widest a layer may be, metres (`blackbox_modding::content::parse_sparks_profile` refuses more).
const MAX_SIZE: f32 = 10.0;
/// The brightness up to which a layer is drawn with alpha blending rather than added.
const DUST_BRIGHTNESS: f32 = 1.5;

/// A number with at most four decimals and no trailing zeros, always with a decimal point (the profiles hold floats).
pub(super) fn num(value: f32) -> String {
    let text = format!("{value:.4}");
    let text = text.trim_end_matches('0').trim_end_matches('.');
    match text {
        "" | "-" | "-0" => "0.0".to_owned(),
        other if other.contains('.') => other.to_owned(),
        other => format!("{other}.0"),
    }
}

fn list(values: &[f32]) -> String {
    let parts: Vec<String> = values.iter().map(|value| num(*value)).collect();
    format!("[{}]", parts.join(", "))
}

fn gate(out: &mut String, section: &str, gate: Gate) {
    let _ = writeln!(out, "\n[{section}]");
    match gate {
        Gate::Impact { floor, full_speed } => {
            let _ = writeln!(
                out,
                "closing_speed_full = {}   # m/s of closing speed that plays the effect at full strength",
                num(full_speed)
            );
            let _ = writeln!(
                out,
                "intensity_floor = {}      # the strength at no closing speed (count and speed are scaled by it)",
                num(floor)
            );
        }
        Gate::Scrape { start, full } => {
            let _ =
                writeln!(out, "speed_start = {}   # m/s of sliding speed below which nothing is thrown", num(start));
            let _ = writeln!(out, "speed_full = {}    # m/s from which the rate is the full one", num(full));
        }
    }
}

fn mean(range: [f32; 2]) -> f32 {
    (range[0] + range[1]) * 0.5
}

/// The share a range's lower end falls short of its upper end by.
fn variance(range: [f32; 2]) -> f32 {
    match range[1] > 0.0 {
        true => (1.0 - range[0] / range[1]).clamp(0.0, 1.0),
        false => 0.0,
    }
}

/// Particles per second (a scrape) or at once (an impact) for a patch of the surface of the spark's radius.
fn count(spark: &Spark) -> f32 {
    spark.density * std::f32::consts::PI * spark.patch_radius * spark.patch_radius * 0.5
}

fn byte(value: f32) -> u32 {
    (value.clamp(0.0, 1.0) * 255.0).round() as u32
}

/// The profile of the original-style emitters (`SPARKS/MW2012/profile.toml`).
pub(super) fn sparks(set: &Set) -> String {
    let mut out = String::from("format = 1\n");
    pc_emitters(&mut out, "hit", "burst", &set.hit);
    pc_emitters(&mut out, "scrape", "rate", &set.scrape);
    out
}

fn pc_emitters(out: &mut String, section: &str, count_key: &str, channel: &Channel) {
    gate(out, section, channel.gate);
    for spark in channel.sparks.iter().take(MAX_EMITTERS) {
        let _ = writeln!(out, "\n[[{section}.layer]]");
        let kind = match spark.kind {
            Kind::Streak => "streak",
            Kind::Sprite => "sprite",
        };
        let _ = writeln!(out, "kind = \"{kind}\"");
        if let Some(sheet) = &spark.sheet {
            let _ = writeln!(out, "texture = \"{}/{}\"", super::SPARKS_FOLDER, sheet.name);
            let cells = match sheet.grid {
                0 | 1 => 0,
                n => n,
            };
            let _ = writeln!(out, "grid = {cells}");
        }
        // A sprite that is not overbright (the grey dust) is a haze, not a light: it covers instead of adding.
        let glows = spark.brightness.iter().any(|brightness| *brightness > DUST_BRIGHTNESS);
        let blend = match glows {
            true => "additive",
            false => "alpha",
        };
        let _ = writeln!(out, "blend = \"{blend}\"");
        let _ = writeln!(out, "{count_key} = {}", num(count(spark).round()));
        let _ = writeln!(out, "life = {}", num(spark.life[1]));
        let _ = writeln!(out, "life_variance = {}", num(variance(spark.life)));
        let _ = writeln!(out, "speed = {}", num(spark.speed[1]));
        let _ = writeln!(out, "speed_variance = {}", num(variance(spark.speed)));
        let _ = writeln!(out, "spread = {}", num(spark.tilt));
        if let Some(bounce) = spark.bounce {
            let _ = writeln!(out, "gravity = {}", num(-bounce.gravity));
            let _ = writeln!(out, "drag = {}", num(pc_drag(bounce.drag, mean(spark.speed))));
            let _ = writeln!(out, "bounce = {}", list(&bounce.restitution));
            let _ = writeln!(out, "friction = {}", list(&bounce.friction));
        }
        let _ = writeln!(out, "inherit = {}", num(spark.inherit[1].min(1.0)));
        let _ = writeln!(out, "inherit_variance = {}", num(variance(spark.inherit)));
        if let Some(stretch) = spark.stretch {
            let _ = writeln!(out, "stretch = {}", num(stretch));
        }
        let _ = writeln!(out, "keys = {}", list(&KEYS));
        // The profile takes at most MAX_SIZE metres; the 2012 dust sprite (3 m radius growing to 1.85 times) is wider.
        let size: Vec<f32> = spark.width.iter().map(|width| width.min(MAX_SIZE)).collect();
        let _ = writeln!(out, "size = {}", list(&size));
        let colors: Vec<String> = (0..KEYS.len())
            .map(|k| {
                let [r, g, b] = spark.rgb[k];
                format!("[{}, {}, {}, {}]", byte(r), byte(g), byte(b), byte(spark.alpha[k]))
            })
            .collect();
        let _ = writeln!(out, "colors = [{}]", colors.join(", "));
        let _ = writeln!(out, "brightness = {}", list(&spark.brightness));
    }
}

/// The host's drag slows a particle by `drag * speed` a second; the 2012 game by a share `loss` of its speed a second.
fn pc_drag(loss: f32, speed: f32) -> f32 {
    match loss > 0.0 && loss < 1.0 && speed > 0.0 {
        true => -(1.0 - loss).ln() / speed,
        false => 0.0,
    }
}

/// The profile of the restored line sparks (`XENON/MW2012/profile.toml`). The contrail and the view streaks have no 2012
/// counterpart in the install and are not in it.
pub(super) fn xenon(set: &Set) -> String {
    let mut out = String::from("format = 1\n");
    lines(&mut out, "hit", &set.hit);
    lines(&mut out, "scrape", &set.scrape);
    out.push_str(
        "\n# No [trail] and no [streaks]: the 2012 data has no wind contrail or speed lines to take them from.\n",
    );
    out
}

fn lines(out: &mut String, section: &str, channel: &Channel) {
    gate(out, section, channel.gate);
    let streaks = channel.sparks.iter().filter(|spark| spark.kind == Kind::Streak && spark.stretch.is_some());
    for spark in streaks.take(MAX_LINES) {
        let _ = writeln!(out, "\n[[{section}.spark]]");
        let alpha = 1.0;
        let [r, g, b] = spark.rgb[0];
        let top = r.max(g).max(b).max(1e-3);
        let _ = writeln!(out, "color = {}", list(&[r / top, g / top, b / top, alpha]));
        let life = mean(spark.life);
        let _ = writeln!(out, "life = {}", num(life));
        let _ = writeln!(out, "life_variance = 0.0");
        let _ = writeln!(out, "count = {}", num(count(spark).round().min(256.0)));
        let _ = writeln!(out, "count_variance = 0.0");
        let _ = writeln!(out, "center = [0.0, 0.0, 0.0]");
        let side = spark.patch_radius.min(1.0);
        let _ = writeln!(out, "extent = {}", list(&[side, side, 0.0]));
        let speed = mean(spark.speed);
        let spread = (spark.tilt.to_radians() * 0.5).tan() * speed;
        let _ = writeln!(out, "velocity = {}", list(&[spread, spread, speed]));
        let swing = match spark.speed[1] + spark.speed[0] > 0.0 {
            true => (spark.speed[1] - spark.speed[0]) / (spark.speed[1] + spark.speed[0]),
            false => 0.0,
        };
        let _ = writeln!(out, "variation = {}", list(&[1.0, 1.0, swing]));
        let share = mean(spark.inherit);
        let _ = writeln!(out, "inherit = {}", list(&[share, share, 0.0]));
        let gravity = spark.bounce.map_or(0.0, |bounce| bounce.gravity * 0.5);
        let _ = writeln!(out, "gravity = {}", num(gravity));
        let _ = writeln!(out, "gravity_delta = 0.0");
        let stretch = spark.stretch.unwrap_or(0.0);
        let _ = writeln!(out, "length = {}", num((stretch * 2048.0).clamp(0.0, 255.0).round()));
        let _ = writeln!(out, "length_delta = 0.0");
        let width = spark.width.iter().sum::<f32>() / spark.width.len() as f32;
        let _ = writeln!(out, "height = {}", num((width * 2048.0).clamp(0.0, 255.0).round()));
        if let Some(bounce) = spark.bounce {
            let _ = writeln!(out, "elasticity = {}", num((mean(bounce.restitution) * 255.0).clamp(0.0, 255.0).round()));
        }
    }
}
