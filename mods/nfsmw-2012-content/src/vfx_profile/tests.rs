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
    /// Peak, base, rise and fall of the radius curve.
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
        }
    }

    fn glow() -> Spec {
        Spec {
            life: (0.25, 0.5),
            radius: (0.1, 0.1),
            curve: [0.5, 1.0, 0.0, 0.0],
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
        }
    }

    fn emitter(&self, width: u32, shade: u8) -> Emitter {
        let [peak, base, rise, fall] = self.curve;
        let mut gradient = block(
            0xA4,
            &[(0x20, self.colour[0]), (0x24, self.colour[1]), (0x28, self.colour[2]), (0x2C, self.colour[3])],
        );
        gradient[0x90..0x95].copy_from_slice(&self.alpha);
        gradient[0x95..0x9A].copy_from_slice(&[0, 26, 77, 179, 255]);
        gradient[0x9C..0xA0].copy_from_slice(&1.0f32.to_le_bytes());
        gradient[0xA0..0xA4].copy_from_slice(&1.0f32.to_le_bytes());
        let mut tasks = vec![
            task(
                ops::EMITTER_CONE_PLUS,
                block(
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
                ),
            ),
            task(ops::INITIALISE_AGE_AND_LIFETIME, block(0x20, &[(0x0C, self.life.0), (0x10, self.life.1)])),
            task(ops::INITIALISE_RADIUS, block(0x14, &[(0x08, self.radius.0), (0x0C, self.radius.1)])),
            task(ops::INITIALISE_VELOCITY_INHERITANCE, block(8, &[(0, 0.5), (4, 1.0)])),
            task(ops::INITIALISE_2D_ROTATION, block(0x18, &[(0, 0.0), (4, 0.5)])),
            task(ops::UPDATE_POS_VEL, block(0x20, &[(0x0C, self.lift), (0x10, self.drag)])),
            task(ops::UPDATE_PARAMETRIC_RADIUS, block(0x28, &[(0x14, peak), (0x18, base), (0x1C, rise), (0x20, fall)])),
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
    assert_eq!((pop.inherit, pop.inherit_variance), (1.0, 0.5), "the share of the car's velocity at birth");
    assert_eq!(pop.angle_range, 180.0, "a half turn of starting rotation");
    assert_eq!((pop.grid, pop.fps), (5, 125), "the 25 frames play over a life of a fifth of a second");
    assert!(pop.additive, "a flipbook adds");
    assert_eq!(pop.texture, "NITROUS/MW2012/FLAME.PNG");
}

#[test]
fn the_size_follows_the_radius_curve_at_the_keys() {
    let set = build(&[], &[], &[Spec::flame().emitter(512, 1)]).unwrap();
    let pop = profile(&set).effects.remove(0);
    // Radius 0.3 on average, width twice that; the curve at 0 is base + peak * fall = 0.625, at the peak 0.5 + 0.25 + 0.125.
    let width = |t: f32| 2.0 * 0.3 * (t.min(0.25) + 0.5 + t.max(0.25) * 0.5);
    for (key, size) in pop.keys.iter().zip(pop.size) {
        assert!((size - width(*key)).abs() < 1e-3, "at {key}: {size} against {}", width(*key));
    }
    assert!(pop.keys.contains(&0.0) && pop.keys.contains(&1.0));
    // Between the keys the host draws a straight line: it stays near the curve everywhere.
    let line = |t: f32| {
        let at = (1..4).find(|i| t <= pop.keys[*i]).unwrap_or(3);
        let (k0, k1) = (pop.keys[at - 1], pop.keys[at]);
        pop.size[at - 1] + (pop.size[at] - pop.size[at - 1]) * (t - k0) / (k1 - k0)
    };
    let worst = (0..=100).map(|n| (line(n as f32 / 100.0) - width(n as f32 / 100.0)).abs()).fold(0.0, f32::max);
    assert!(worst < 0.08 * width(1.0), "the four keys {:?} miss the curve by {worst}", pop.keys);
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
    assert_eq!((glow.duration, glow.rate, glow.rate_per_speed), (0.0, 5.0, 1.0));
    assert_eq!((glow.speed, glow.spread), (0.0, 0.0), "it does not move");
    assert!(glow.additive && glow.grid == 0);
    assert_eq!(glow.texture, "NITROUS/MW2012/GLOW.PNG");
    assert_eq!(glow.damping, 0.75, "the motion's drag keeps taking a share of the velocity: the glow is left behind");
    let at_peak = glow.colors.iter().find(|c| c[3] == 255).expect("a full alpha key");
    assert_eq!(&at_peak[..3], &[8, 133, 219], "the blue read from the gradient");
}

#[test]
fn particles_that_follow_their_emitter_ride_with_the_car_and_are_not_damped() {
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
    // 0.17 and that between those of 0.7 by 0.19, so the point at 0.3 is the one dropped.
    let through = [(0.0, 0.0), (0.1, 1.0), (0.3, 0.9), (0.7, 0.2), (1.0, 0.0)];
    let curve = |t: f32| {
        let at = through.windows(2).find(|w| t <= w[1].0).unwrap();
        at[0].1 + (at[1].1 - at[0].1) * (t - at[0].0) / (at[1].0 - at[0].0)
    };
    let keys = choose_keys(&[0.0, 0.1, 0.3, 0.7, 1.0], |t| [curve(t), 0.0, 0.0, 0.0, 0.0]);
    assert_eq!(keys, [0.0, 0.1, 0.7, 1.0]);
    // Too few knots: the widest gap is split.
    assert_eq!(choose_keys(&[0.0, 1.0], |_| [0.0; 5]), [0.0, 0.5, 0.75, 1.0]);
}

#[test]
#[ignore = "reads the 2012 install (MW2012_GAME_DIR)"]
fn the_real_effects_become_a_profile_the_game_reads() {
    use chameleon_bundle::Bundle;
    let root = std::path::PathBuf::from(std::env::var_os("MW2012_GAME_DIR").expect("MW2012_GAME_DIR"));
    let bytes = std::fs::read(root.join(crate::vfx::GLOBAL_EFFECTS)).unwrap();
    let effects = Bundle::parse(&bytes).unwrap();
    let namespace = 1u64 << 56;
    let [start, burn, pop] =
        [0x16205B_u64, 0x22030B, 0x24F2D].map(|id| crate::vfx::emitters(&effects, namespace | id).unwrap());
    let set = build(&start, &burn, &pop).expect("something to play");
    println!("{}", set.toml);
    let effects = profile(&set).effects;
    let on = |moment: EffectOn| effects.iter().filter(|e| e.on == moment).count();
    assert!(on(EffectOn::Start) >= 3 && on(EffectOn::Burn) >= 1 && on(EffectOn::Pop) >= 2, "{set:?}");
}

/// The pipes the host plays the effects at (the converted car's EXHAUST markers, whose +z is the way the flame shoots), on the
/// whole install: every pipe is in the rear half of its car and none shoots forwards. Most shoot straight back; a few cars
/// really have side exits (the Cobra, the Viper, the Caterham, the M3 GTR, the 918 concept: their data says so), and those
/// shoot sideways. The 918 Spyder (2013) has its pipes on the rear deck and the Gallardo and the BMW 1M theirs at the rear
/// face: the effect's origin is at the pipe and its way is back, `-x` of the car.
#[test]
#[ignore = "reads the 2012 install (MW2012_GAME_DIR)"]
fn every_pipe_of_the_install_is_at_the_rear_and_none_shoots_forwards() {
    use crate::fit::to_car_space;
    use crate::pipes::{Pipes, exhaust_markers};
    use crate::source::Source;
    let root = std::path::PathBuf::from(std::env::var_os("MW2012_GAME_DIR").expect("MW2012_GAME_DIR"));
    let source = Source::open(&root).unwrap();
    let (mut cars, mut odd, mut straight_back) = (0, Vec::new(), Vec::new());
    for file in source.cars() {
        let car = source.load(file).unwrap();
        let name = source.object_name(file).unwrap_or_else(|| format!("VEH_{}", file.number));
        let Pipes::Markers(found) = exhaust_markers(&car.lamps, &to_car_space) else { continue };
        cars += 1;
        for marker in &found {
            let [x, _, _] = marker.translation();
            let way = marker.matrix[8];
            if x >= 0.0 || way > 0.1 {
                odd.push(format!("{name}: pipe at {:?} shoots {way:.2} along the car", marker.translation()));
            }
            if way < -0.99 {
                straight_back.push(name.clone());
            }
        }
    }
    println!("{cars} cars with pipes; not at the rear or shooting forwards: {odd:#?}");
    assert!(cars > 80 && odd.is_empty(), "{odd:#?}");
    for name in ["Porsche_918Spyder_2013", "Lamborghini_GallardoLP5704SpyderPerformante_2011", "BMW_1M_2012"] {
        assert!(straight_back.iter().any(|known| known == name), "{name} shoots straight back");
    }
}
