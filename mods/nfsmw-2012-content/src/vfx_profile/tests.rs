use blackbox_modding::content::{EffectOn, parse_nitrous_profile};
use chameleon_effects::{Task, ops};

use super::*;

fn picture(width: u32, shade: u8) -> Picture {
    Picture { width, height: width, rgba: vec![shade; (width * width * 4) as usize] }
}

fn block(size: usize, floats: &[(usize, f32)]) -> Vec<u8> {
    let mut bytes = vec![0u8; size];
    for (at, value) in floats {
        bytes[*at..*at + 4].copy_from_slice(&value.to_le_bytes());
    }
    bytes
}

/// The numbers a test emitter is made of.
struct Spec {
    life: (f32, f32),
    radius: (f32, f32),
    /// The peak, and the radius factor at birth, at the peak and at the end.
    curve: [f32; 4],
    velocity: f32,
    window_end: f32,
    rate: (f32, f32),
    lift: f32,
    drag: f32,
    /// The particles follow the emitter.
    follows: bool,
    /// The first colour, its brightness, and the alpha curve's heights at its five points.
    colour: [f32; 4],
    alpha: [u8; 5],
    flipbook: bool,
    render: u8,
    /// The index of the parameter that scales the spawn rate.
    rate_param: u8,
}

impl Spec {
    fn flame() -> Spec {
        Spec {
            life: (0.1, 0.2),
            radius: (0.2, 0.4),
            curve: [0.25, 0.5, 1.0, 0.5],
            velocity: 4.0,
            window_end: 0.5,
            rate: (100.0, 0.0),
            lift: 10.0,
            drag: 0.0,
            follows: false,
            colour: [1.0, 0.5, 0.25, 2.0],
            alpha: [0, 255, 200, 100, 0],
            flipbook: true,
            render: ops::RENDER_SPRITE_SUB_UV,
            rate_param: 1,
        }
    }

    fn glow() -> Spec {
        Spec {
            life: (0.25, 0.5),
            radius: (0.1, 0.1),
            curve: [0.5, 1.0, 1.0, 1.0],
            velocity: 0.0,
            window_end: 1.0e9,
            rate: (5.0, 1.0),
            lift: 0.0,
            drag: 0.75,
            follows: false,
            colour: [0.03, 0.52, 0.86, 1.0],
            alpha: [0, 255, 255, 128, 0],
            flipbook: false,
            render: ops::RENDER_SPRITE,
            rate_param: 0,
        }
    }

    fn emitter(&self, width: u32, shade: u8) -> Emitter {
        let [peak, at_start, at_peak, at_end] = self.curve;
        let mut gradient = block(
            0xA4,
            &[(0x20, self.colour[0]), (0x24, self.colour[1]), (0x28, self.colour[2]), (0x2C, self.colour[3])],
        );
        gradient[0x90..0x95].copy_from_slice(&self.alpha);
        gradient[0x95..0x9A].copy_from_slice(&[0, 26, 77, 179, 255]);
        gradient[0x9C..0xA0].copy_from_slice(&1.0f32.to_le_bytes());
        gradient[0xA0..0xA4].copy_from_slice(&1.0f32.to_le_bytes());
        let mut cone = block(
            0x80,
            &[
                (0, self.velocity),
                (4, self.velocity),
                (8, self.velocity),
                (0x10, 0.0),
                (0x14, self.window_end),
                (0x60, self.rate.0),
                (0x64, self.rate.1),
            ],
        );
        cone[0x70] = self.rate_param;
        let mut tasks = vec![
            task(ops::EMITTER_CONE_PLUS, cone),
            task(ops::INITIALISE_AGE_AND_LIFETIME, block(0x20, &[(0x0C, self.life.0), (0x10, self.life.1)])),
            task(ops::INITIALISE_RADIUS, block(0x14, &[(0x08, self.radius.0), (0x0C, self.radius.1)])),
            task(ops::INITIALISE_VELOCITY_INHERITANCE, block(8, &[(0, 0.5), (4, 1.0)])),
            task(ops::INITIALISE_2D_ROTATION, block(0x18, &[(0, 0.0), (4, 0.5)])),
            task(ops::UPDATE_POS_VEL, block(0x20, &[(0x0C, self.lift), (0x10, self.drag)])),
            task(
                ops::UPDATE_PARAMETRIC_RADIUS,
                block(0x28, &[(0x14, peak), (0x18, at_start), (0x1C, at_peak), (0x20, at_end)]),
            ),
            task(ops::PARAMETRIC_SET_COLOUR_GRADIENT, gradient),
        ];
        if self.flipbook {
            tasks.push(task(ops::PARAMETRIC_SET_SUB_UV, Vec::new()));
        }
        if self.follows {
            tasks.push(task(ops::PARAMETRIC_POS_VEL_TO_WORLD_SPACE_WITH_LOCAL_TRANSFORM, Vec::new()));
        }
        tasks.push(task(self.render, Vec::new()));
        Emitter {
            picture: picture(width, shade),
            flipbook: self.flipbook,
            fx: chameleon_effects::Emitter { flags: 0, tasks, stages: Default::default() },
        }
    }
}

fn task(op: u8, block: Vec<u8>) -> Task {
    Task { op, block }
}

fn profile(set: &EffectSet) -> blackbox_modding::content::NitrousProfile {
    let text =
        format!("format = 1\n[flame]\nreplace_burn = {}\nreplace_pop = {}\n{}", set.has_burn, set.has_pop, set.toml);
    parse_nitrous_profile(&text).unwrap_or_else(|e| panic!("{e}\n{text}"))
}

#[test]
fn a_flame_takes_its_numbers_from_the_emitter() {
    let set = build(&[], &[], &[Spec::flame().emitter(512, 1)]).unwrap();
    let pop = profile(&set).effects.remove(0);
    assert_eq!(pop.on, EffectOn::Pop);
    assert_eq!((pop.rate, pop.rate_per_speed, pop.duration), (100.0, 0.0, 0.5), "the half second window");
    assert!((pop.life - 0.2).abs() < 1e-6 && (pop.life_variance - 0.5).abs() < 1e-6, "{pop:?}");
    assert_eq!((pop.speed, pop.speed_variance, pop.lift, pop.damping), (4.0, 1.0, 10.0, 0.0));
    assert_eq!((pop.inherit, pop.inherit_variance), (1.0, 0.0), "it rides with the car");
    assert_eq!(pop.angle_range, 180.0, "a half turn of starting rotation");
    assert_eq!((pop.grid, pop.fps), (5, 125), "the 25 frames play over a life of a fifth of a second");
    assert!(pop.additive, "a flipbook adds");
    assert_eq!(pop.texture, "NITROUS/MW2012/FLAME.PNG");
}

#[test]
fn the_size_follows_the_radius_curve_at_the_keys() {
    let set = build(&[], &[], &[Spec::flame().emitter(512, 1)]).unwrap();
    let pop = profile(&set).effects.remove(0);
    // Radius 0.3 on average, width twice that; the factor goes 0.5, 1 at the peak (0.25), 0.5 at the end.
    let width = |t: f32| 2.0 * 2.0 * 0.3 * if t <= 0.25 { 0.5 + 2.0 * t } else { 1.0 - 0.5 * (t - 0.25) / 0.75 };
    for (key, size) in pop.keys.iter().zip(&pop.size) {
        assert!((size - width(*key)).abs() < 1e-3, "at {key}: {size} against {}", width(*key));
    }
    assert!(pop.keys.contains(&0.0) && pop.keys.contains(&1.0));
    assert!(pop.keys.iter().any(|k| (k - 0.25).abs() < 1e-3), "the curve's peak is a key: {:?}", pop.keys);
    // Between the keys the host draws a straight line: it stays on the curve everywhere.
    let line = |t: f32| {
        let at = (1..pop.keys.len()).find(|i| t <= pop.keys[*i]).unwrap_or(pop.keys.len() - 1);
        let (k0, k1) = (pop.keys[at - 1], pop.keys[at]);
        pop.size[at - 1] + (pop.size[at] - pop.size[at - 1]) * (t - k0) / (k1 - k0)
    };
    let worst = (0..=100).map(|n| (line(n as f32 / 100.0) - width(n as f32 / 100.0)).abs()).fold(0.0, f32::max);
    assert!(worst < 0.01, "the keys {:?} miss the curve by {worst}", pop.keys);
}

#[test]
fn a_bright_colour_is_multiplied_in_and_the_alpha_follows_the_curve() {
    let set = build(&[], &[], &[Spec::flame().emitter(512, 1)]).unwrap();
    let pop = profile(&set).effects.remove(0);
    let peak = pop.keys.iter().position(|k| (k - 0.1).abs() < 1e-2).expect("the alpha peak is a key");
    assert_eq!(
        pop.colors[peak],
        [255, 255, 128, 255],
        "red 1 and green 0.5 times a brightness of 2, and the alpha peak"
    );
    assert_eq!(pop.colors[0][3], 0, "it starts invisible");
}

#[test]
fn the_glow_burns_for_as_long_as_the_nitrous_does_and_stays_on_the_pipe() {
    let set = build(&[], &[Spec::glow().emitter(256, 2)], &[]).unwrap();
    assert!(set.has_burn && !set.has_pop);
    let glow = profile(&set).effects.remove(0);
    assert_eq!((glow.duration, glow.rate), (0.0, 5.0));
    assert!((glow.rate_per_speed - 1.0 / 60.0).abs() < 1e-6, "the term is read as the movement of a frame");
    assert_eq!((glow.speed, glow.spread), (0.0, 0.0), "it does not move");
    assert!(glow.additive && glow.grid == 0);
    assert_eq!(glow.texture, "NITROUS/MW2012/GLOW.PNG");
    assert_eq!((glow.damping, glow.inherit), (0.0, 1.0), "it rides with the car, as the references show it");
    let at_peak = glow.colors.iter().find(|c| c[3] == 255).expect("a full alpha key");
    assert_eq!(&at_peak[..3], &[8, 133, 219], "the blue read from the gradient");
}

#[test]
fn every_sprite_rides_with_the_car_and_is_not_damped() {
    let mut follower = Spec::flame();
    follower.follows = true;
    follower.drag = 0.5;
    let set = build(&[follower.emitter(512, 1)], &[], &[]).unwrap();
    let flame = profile(&set).effects.remove(0);
    assert_eq!((flame.inherit, flame.inherit_variance, flame.damping), (1.0, 0.0, 0.0));
}

#[test]
fn an_open_window_of_a_start_effect_is_as_long_as_the_effects_other_spawners() {
    let mut open = Spec::flame();
    open.window_end = 1.0e9;
    let mut closed = Spec::flame();
    closed.window_end = 0.8;
    let set = build(&[open.emitter(512, 1), closed.emitter(512, 1)], &[], &[]).unwrap();
    let durations: Vec<f32> = profile(&set).effects.iter().map(|e| e.duration).collect();
    assert_eq!(durations, [0.8, 0.8]);
    let alone = build(&[open.emitter(512, 1)], &[], &[]).unwrap();
    assert!(
        (profile(&alone).effects[0].duration - 0.2).abs() < 1e-6,
        "no other window: one generation, the longest life"
    );
    let mut burning = Spec::glow();
    burning.window_end = 1.0e9;
    let burn = build(&[], &[burning.emitter(256, 2)], &[]).unwrap();
    assert_eq!(profile(&burn).effects[0].duration, 0.0, "a burn lasts as long as the nitrous does");
}

#[test]
fn each_picture_is_written_once_and_a_different_one_gets_its_own_name() {
    let mut smoke = Spec::glow();
    smoke.window_end = 0.5;
    let set = build(
        &[smoke.emitter(128, 3), smoke.emitter(128, 3), smoke.emitter(128, 4), Spec::flame().emitter(512, 1)],
        &[Spec::glow().emitter(256, 2)],
        &[Spec::flame().emitter(512, 1), Spec::flame().emitter(128, 5)],
    )
    .unwrap();
    let names: Vec<&str> = set.pictures.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(names, ["SMOKE.PNG", "SMOKE2.PNG", "FLAME.PNG", "GLOW.PNG", "DUST.PNG"]);
    assert_eq!(profile(&set).effects.len(), 7);
}

#[test]
fn the_puffs_that_a_crash_input_scales_are_left_out() {
    let mut puff = Spec::glow();
    puff.rate_param = 3;
    assert!(build(&[puff.emitter(128, 1)], &[], &[]).is_none(), "scaled by a crash input: zero when driving");
    puff.rate_param = 1;
    assert!(build(&[puff.emitter(128, 1)], &[], &[]).is_some());
}

#[test]
fn a_refraction_sprite_and_an_emitter_missing_its_blocks_are_left_out() {
    let mut haze = Spec::glow();
    haze.render = ops::RENDER_SPRITE_UV_DISTORTION;
    let mut bare = Spec::glow().emitter(128, 1);
    bare.fx.tasks.retain(|t| t.op != ops::PARAMETRIC_SET_COLOUR_GRADIENT);
    assert!(build(&[haze.emitter(128, 1), bare], &[], &[]).is_none());
}

#[test]
fn the_keys_keep_the_ends_and_drop_the_point_the_curves_miss_least() {
    // A curve through (0, 0), (0.1, 1), (0.3, 0.9), (0.7, 0.2) and (1, 0): the line between the neighbours of 0.3 misses it by
    // 0.17 and that between those of 0.7 by 0.19, so with room for four the point at 0.3 is the one dropped.
    let through = [(0.0, 0.0), (0.1, 1.0), (0.3, 0.9), (0.7, 0.2), (1.0, 0.0)];
    let curve = |t: f32| {
        let at = through.windows(2).find(|w| t <= w[1].0).unwrap();
        at[0].1 + (at[1].1 - at[0].1) * (t - at[0].0) / (at[1].0 - at[0].0)
    };
    let sample = |t: f32| [curve(t), 0.0, 0.0, 0.0, 0.0];
    assert_eq!(choose_keys(&[0.0, 0.1, 0.3, 0.7, 1.0], 4, sample), [0.0, 0.1, 0.7, 1.0]);
    assert_eq!(choose_keys(&[0.0, 0.1, 0.3, 0.7, 1.0], 12, sample).len(), 5, "with room, every knot is a key");
    assert_eq!(choose_keys(&[], 4, |_| [0.0; 5]), [0.0, 1.0], "the ends are always keys");
}
