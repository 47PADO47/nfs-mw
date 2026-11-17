use super::super::super::car_light_flares::emergency;
use super::super::super::car_lights::Lamp;
use super::*;
use std::f32::consts::PI;

/// A bar with a red lamp on the left, a blue one on the right and a white one between, 1.5 m up.
fn bar() -> CarLights {
    let lamp = |at, kind| Lamp { at, direction: Vec3::X, kind, flags: 0 };
    CarLights::new(vec![
        lamp(Vec3::new(0.0, 0.4, 1.5), Kind::Siren(true)),
        lamp(Vec3::new(0.0, -0.4, 1.5), Kind::Siren(false)),
        lamp(Vec3::new(0.0, 0.0, 1.5), Kind::SirenWhite),
        lamp(Vec3::new(2.0, 0.6, 0.6), Kind::Head),
    ])
}

/// The bar running, at `time` (the red half of the pulse at 0, the blue half at pi/8).
fn running(time: f32) -> LampState {
    LampState { emergency: emergency::RED | emergency::BLUE | emergency::WHITE, time, ..Default::default() }
}

fn emit(lamps: &CarLights, transform: Mat4, state: LampState) -> Vec<LocalLight> {
    let mut out = Vec::new();
    super::emit(lamps, transform, state, 1.0, &mut out);
    out
}

fn reddest(lights: &[LocalLight]) -> Option<&LocalLight> {
    lights.iter().find(|l| l.color[0] > l.color[2] * 4.0)
}

fn bluest(lights: &[LocalLight]) -> Option<&LocalLight> {
    lights.iter().find(|l| l.color[2] > l.color[0] * 4.0)
}

#[test]
fn the_lit_colour_follows_the_flare_pulse() {
    let at = Mat4::from_translation(Vec3::new(10.0, 20.0, 0.0));
    let red = emit(&bar(), at, running(0.0));
    let light = reddest(&red).expect("the red half of the pulse lights red");
    assert!(bluest(&red).is_none(), "blue is dark while red flashes");
    assert_eq!(light.position, [10.0, 20.4, 1.5 + LIFT]);
    assert_eq!(light.direction, [0.0, 0.0, -1.0]);
    let blue = emit(&bar(), at, running(PI / 8.0));
    assert!(bluest(&blue).is_some() && reddest(&blue).is_none(), "the other half lights blue only");
    for light in red.iter().chain(&blue) {
        assert!(light.valid(), "{light:?}");
        assert!(!light.casts_shadow && light.upper_plane.is_none());
        assert!((10.0..=20.0).contains(&light.range));
    }
    assert!(red.len() <= 3, "at most one light per colour, not per lamp");
}

#[test]
fn a_tilted_car_points_its_light_down_its_own_axis() {
    let tilt = Mat4::from_rotation_x(0.3);
    let lights = emit(&bar(), tilt, running(0.0));
    let down = tilt.transform_vector3(Vec3::NEG_Z);
    assert!(lights.iter().all(|l| Vec3::from(l.direction).distance(down) < 1e-5));
}

#[test]
fn nothing_is_cast_while_the_bar_is_off_broken_dark_or_weightless() {
    let off = LampState { emergency: 0, ..running(0.0) };
    assert!(emit(&bar(), Mat4::IDENTITY, off).is_empty());
    let broken = LampState { broken: emergency::RED, ..running(0.0) };
    assert!(reddest(&emit(&bar(), Mat4::IDENTITY, broken)).is_none());
    assert!(emit(&bar(), Mat4::IDENTITY, LampState { dark: true, ..running(0.0) }).is_empty());
    let mut out = Vec::new();
    super::emit(&bar(), Mat4::IDENTITY, running(0.0), 0.0, &mut out);
    assert!(out.is_empty(), "zero strength casts nothing");
    let plain = CarLights::new(vec![Lamp { at: Vec3::ZERO, direction: Vec3::X, kind: Kind::Head, flags: 0 }]);
    assert!(emit(&plain, Mat4::IDENTITY, running(0.0)).is_empty(), "a car without a bar");
}

#[test]
fn the_nearest_bars_light_within_range_and_budget() {
    let lamps = bar();
    let bars = |xs: &[f32]| {
        xs.iter()
            .enumerate()
            .map(|(key, &x)| LightBar {
                key: key as u32,
                lamps: &lamps,
                transform: Mat4::from_translation(Vec3::new(x, 0.0, 0.0)),
                state: running(0.0),
            })
            .collect::<Vec<_>>()
    };
    let nearest = |xs: &[f32], strength: f32, remaining: usize| {
        let mut out = Vec::new();
        emit_nearest(bars(xs).into_iter(), Vec3::ZERO, strength, remaining, &mut out);
        out
    };
    let one = emit(&lamps, Mat4::IDENTITY, running(0.0)).len();
    assert!(one > 0);
    assert!(nearest(&[DISTANCE + 5.0], 1.0, 128).is_empty(), "out of range");
    assert!(nearest(&[5.0], 1.0, 0).is_empty(), "the budget is spent");
    assert!(nearest(&[5.0], 0.0, 128).is_empty(), "the option's strength is zero");
    assert_eq!(nearest(&[5.0], 1.0, 1).len(), 1, "the light budget cuts the list");
    let many = nearest(&[70.0, 5.0, 60.0, 10.0, 15.0, 20.0], 1.0, 128);
    assert_eq!(many.len(), CARS * one, "only the nearest cars cast light");
    assert!(many.iter().all(|l| l.position[0] <= 20.0), "nearest first");
    assert!(many.iter().all(LocalLight::valid));
    let edge = nearest(&[DISTANCE - 1.0], 1.0, 128);
    let near = nearest(&[5.0], 1.0, 128);
    assert!(edge[0].color[0] < near[0].color[0] * 0.2, "fades towards the distance boundary");
}
