//! What the car sound publishes to the mixer map each frame. Spec: `docs/specs/car-sound-mixer.md` §2.

use blackbox_carsound::{CarInput, EffectSignals, EngineOutput, ShiftState};
use blackbox_mixmap::Mixer;

use super::ids::{controller, object, player_controller, player_object_input};

/// Unity in the map's Q15 inputs.
const FULL: i32 = 0x7FFF;
/// The audio upgrade level of the engine: level 2 in the original (`m_EngUGL`), 21844 of 32767.
const ENGINE_UPGRADE_LEVEL: i32 = 21_844;
/// Where the rear position object sits behind the car, and how far a wheel is from the car's axis, metres.
const REAR_OFFSET: f32 = 2.0;
const WHEEL_SIDE: f32 = 0.9;
/// The wind sources' circle: radius at standstill, the wind speed range (m/s), the least radius, the angle they
/// start at and how far it opens with speed (of 65536).
const WIND_RADIUS: f32 = 65.0;
const WIND_SLOWEST: f32 = 2.0;
const WIND_FASTEST: f32 = 40.0;
const WIND_MIN_RADIUS: f32 = 3.0;
const WIND_ANGLE_MIN: f32 = 1280.0;
const WIND_ANGLE_SPAN: f32 = 12288.0;

/// What one frame publishes.
pub struct Frame<'a> {
    pub input: &'a CarInput,
    pub engine: &'a EngineOutput,
    pub effects: EffectSignals,
    /// `engineaudio.Master_Vol` of the car's sound set.
    pub master_volume: u32,
    /// The spark chatter is sounding (its module reported a volume the last tick).
    pub sparks: bool,
}

/// Where a 3D object is, for the controls that roll it off.
#[derive(Debug, Clone, Copy)]
struct Place {
    to_car_cm: i32,
    to_camera_cm: i32,
    azimuth_car: i32,
    azimuth_camera: i32,
}

impl Place {
    /// A point at `(x, y)` metres from the car (x ahead, y to the left) seen from a camera `camera` metres behind it.
    fn at(x: f32, y: f32, view: &crate::audio::AudioView) -> Self {
        let [to_car_cm, to_camera_cm, azimuth_car, azimuth_camera] = view.distances(view.point(x, y));
        Place { to_car_cm, to_camera_cm, azimuth_car, azimuth_camera }
    }
}

fn set(m: &mut Mixer, controller: u8, index: u8, value: i32) {
    m.set_input(player_controller(controller, index), value);
}

fn flag(on: bool) -> i32 {
    if on { FULL } else { 0 }
}

fn q15(value: f32) -> i32 {
    (value as i32).clamp(0, FULL)
}

pub fn publish(m: &mut Mixer, f: &Frame<'_>, view: &crate::audio::AudioView) {
    crate::audio::speedbreaker::publish(m, view);
    let (input, engine) = (f.input, f.engine);
    physics(m, input, engine, view);
    set(m, controller::ENGINE, 0, ENGINE_UPGRADE_LEVEL);
    set(m, controller::ENGINE, 1, flag(engine.events.compression_bump));
    set(m, controller::ENGINE, 2, f.master_volume as i32);
    set(m, controller::HYBRID, 0, q15(engine.steady_signal));
    set(m, controller::HYBRID, 1, q15(engine.change_signal));
    let object_input = |m: &mut Mixer, object: u8, index: u8, value: i32| {
        m.set_input(player_object_input(object, index), value);
    };
    object_input(m, object::SHIFT, 7, flag(engine.shift_state == ShiftState::UpDisengage));
    object_input(m, object::SPARKS, 0, flag(f.sparks));
    object_input(m, object::SPARKS, 2, flag(f.sparks));
    object_input(m, object::NITROUS, 1, flag(input.nos_active));
    object_input(m, object::NITROUS, 2, flag(input.nos_empty));
    let skids = f.effects;
    object_input(m, object::SKIDS, 0, q15(skids.skid_forward));
    object_input(m, object::SKIDS, 2, q15(skids.skid_forward));
    object_input(m, object::SKIDS, 1, q15(skids.skid_side));
    object_input(m, object::SKIDS, 3, q15(skids.skid_load));
    object_input(m, object::ROAD, 0, flag(skids.road_changed));
    positions(m, view, input.speed.abs());
}

fn physics(m: &mut Mixer, input: &CarInput, engine: &EngineOutput, view: &crate::audio::AudioView) {
    let mph = input.speed_mph();
    for (index, scale) in [1092.2334, 546.1167, 327.67, 234.05].into_iter().enumerate() {
        set(m, controller::PHYSICS, index as u8, q15(mph * scale));
    }
    set(m, controller::PHYSICS, 4, q15((10_000.0 - engine.physics_rpm) * 3.640_777_8));
    set(m, controller::PHYSICS, 5, q15(input.wheels_on_ground() as f32 / 4.0 * FULL as f32));
    // The chase camera: neither the bumper (0) nor the hood (4000) view.
    set(m, controller::PHYSICS, 6, view.view_value);
    set(m, controller::PHYSICS, 10, flag(engine.accelerating));
}

fn positions(m: &mut Mixer, view: &crate::audio::AudioView, speed: f32) {
    let speed = if speed > WIND_FASTEST { 39.95 } else { speed };
    let ratio = (speed.clamp(WIND_SLOWEST, WIND_FASTEST)) / WIND_FASTEST;
    let radius = ((1.0 - ratio) * WIND_RADIUS).max(WIND_MIN_RADIUS);
    let spread = (WIND_ANGLE_MIN + ratio * WIND_ANGLE_SPAN) / 65536.0 * std::f32::consts::TAU;
    let wind = |side: f32| Place::at(radius * spread.cos(), side * radius * spread.sin(), view);
    let places = [
        (controller::CAR_POSITION, Place::at(0.0, 0.0, view)),
        (controller::REAR_POSITION, Place::at(-REAR_OFFSET, 0.0, view)),
        (controller::OBJECT_POSITION, Place::at(0.0, 0.0, view)),
        (controller::RIGHT_WHEEL, Place::at(0.0, -WHEEL_SIDE, view)),
        (controller::LEFT_WHEEL, Place::at(0.0, WHEEL_SIDE, view)),
        (controller::LEFT_WIND, wind(1.0)),
        (controller::RIGHT_WIND, wind(-1.0)),
    ];
    for (controller, place) in places {
        set(m, controller, 0, place.to_car_cm);
        set(m, controller, 1, place.to_camera_cm);
        set(m, controller, 2, place.azimuth_car);
        set(m, controller, 3, place.azimuth_camera);
        set(m, controller, 15, 1);
    }
}
