//! Class contracts and sequential Q15 arithmetic; spec: car-audio-power-effects.md.
use blackbox_carsound::{SoundRef, SweetenerKind};
use nfsmw_data::sound::CarSound;

use super::{PowerFrame, PowerLevels};

pub(super) fn q15(a: i32, b: i32) -> i32 {
    ((i64::from(a) * i64::from(b)) >> 15) as i32
}

pub(super) fn cosine(index: i32) -> i32 {
    (32767.0 * (f64::from(index.clamp(0, 513)) * std::f64::consts::PI / 1022.0).cos() + 1e-9).floor().max(0.0) as i32
}

pub(super) fn whine(frame: PowerFrame, levels: PowerLevels) -> [i32; 9] {
    let volume = match frame.max_physics_rpm.is_finite() && frame.max_physics_rpm > 0.0 {
        true => (levels.whine_volume as f32 * frame.physics_rpm / frame.max_physics_rpm) as i32,
        false => 0,
    };
    [
        0,
        (frame.physics_rpm as i32).clamp(0, 10000),
        volume.clamp(0, 32787),
        levels.shift_azimuth.clamp(0, 65536),
        0,
        0,
        25000,
        0,
        32767,
    ]
}

pub(super) fn turbo(id: i32, volume: i32, psi: i32, azimuth: i32, rotation: i32, rpm: f32) -> [i32; 6] {
    [
        id.clamp(0, 5),
        volume.clamp(0, 32767),
        psi.clamp(0, 1024),
        azimuth.clamp(0, 65535),
        rotation.clamp(0, 1024),
        (rpm as i32).clamp(0, 10000),
    ]
}

pub(super) fn nitrous(levels: PowerLevels, stop: bool) -> [i32; 9] {
    [
        0,
        levels.nitrous_volume.clamp(0, 32767),
        levels.nitrous_azimuth.clamp(0, 65535),
        i32::from(stop),
        levels.nitrous_pitch.clamp(0, 16384),
        levels.nitrous_low_pass.clamp(0, 25000),
        0,
        32767,
        0,
    ]
}

pub(super) fn purge(levels: PowerLevels) -> [i32; 8] {
    [levels.purge_volume.clamp(0, 32767), levels.nitrous_azimuth.clamp(0, 65535), 0, 4096, 25000, 0, 32767, 0]
}

pub(super) fn clunk(base: u32, rpm: f32, mixer: i32) -> i32 {
    let rpm = rpm as i32;
    let scale = 0.1 + 0.9 * ((rpm as f32 - 1500.0) / 6500.0).clamp(0.0, 1.0);
    let index = (512.0 - scale * 512.0) as i32;
    q15(q15(base.min(i32::MAX as u32) as i32, cosine(index)), mixer.clamp(0, 32767)).clamp(0, 32767)
}

pub(super) fn sweet(base: i32, rpm: f32, mixer: i32) -> i32 {
    let slope = (26000.0 + (rpm - 4000.0).clamp(0.0, 3000.0) * (6767.0 / 3000.0)) as i32;
    q15(q15(base.max(0), slope), mixer.clamp(0, 32767)).clamp(0, 32787)
}

pub(super) fn shot(
    sound: &CarSound,
    frame: PowerFrame,
    levels: PowerLevels,
    event: SoundRef,
) -> Option<(usize, &'static str, Vec<i32>)> {
    let cam = i32::from(frame.camera_pov_zero);
    let shift = |id, volume| vec![id, volume, 4096, levels.shift_azimuth.clamp(0, 65535), 0, cam];
    match event {
        SoundRef::GearClunk { up } => {
            let volume = match up {
                true => clunk(sound.shift.up_volume, frame.physics_rpm, levels.shift_volume[0]),
                false => clunk(sound.shift.down_volume, frame.physics_rpm, levels.shift_volume[1]),
            };
            Some((0, "FX_SHIFTING_01", shift(i32::from(up), volume)))
        }
        SoundRef::BrakeMash => Some((1, "FX_SHIFTING_01", shift(2, levels.shift_volume[6].clamp(0, 32767)))),
        SoundRef::Sweetener(kind) => {
            let (slot, level, rpm) = match kind {
                SweetenerKind::Disengage => (2, 3, frame.physics_rpm),
                SweetenerKind::Engage => (3, 2, frame.rpm_at_shift),
                SweetenerKind::Accelerate => (4, 4, frame.rpm_at_shift),
                SweetenerKind::EngineOff => (5, 5, frame.rpm_at_shift),
            };
            let words = vec![
                i32::from(kind.sample()),
                sound.engine.car_id.min(100) as i32,
                (frame.physics_rpm as i32).clamp(0, 10000),
                sweet(sound.engine.shift_sweet_volume, rpm, levels.shift_volume[level]),
                levels.shift_azimuth.clamp(0, 65536),
                0,
                0,
            ];
            Some((slot, "CAR_SWTN", words))
        }
        _ => None,
    }
}
