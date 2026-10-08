//! Applying the mixer's levels to the effects mixer's commands.

use blackbox_carsound::{LoopId, SoundCommand, SoundRef, SweetenerKind};

use super::Levels;

/// The gain and the pitch ratio of the mixer map for a sound (loop `axle` is the skid axle: 0 front, 1 rear).
fn level_of(levels: &Levels, sound: SoundRef, skid_axle: Option<u8>) -> (f32, f32) {
    match sound {
        SoundRef::GearClunk { up: true } => (levels.clunk_up, 1.0),
        SoundRef::GearClunk { up: false } => (levels.clunk_down, 1.0),
        SoundRef::BrakeMash => (levels.brake_mash, 1.0),
        SoundRef::Sweetener(SweetenerKind::Engage) => (levels.sweet_engage, 1.0),
        SoundRef::Sweetener(SweetenerKind::Disengage) => (levels.sweet_disengage, 1.0),
        SoundRef::Sweetener(SweetenerKind::Accelerate) => (levels.sweet_accelerate, 1.0),
        SoundRef::Sweetener(SweetenerKind::EngineOff) => (levels.sweet_engine_off, 1.0),
        SoundRef::ReverseWhine => (levels.whine, 1.0),
        SoundRef::TurboSpool => (levels.turbo_spool, 1.0),
        SoundRef::TurboBlowoff(0) => (levels.turbo_blowoff_first, 1.0),
        SoundRef::TurboBlowoff(_) => (levels.turbo_blowoff_other, 1.0),
        SoundRef::Nitrous => (levels.nitrous, levels.nitrous_pitch),
        SoundRef::Purge => (levels.purge, 1.0),
        SoundRef::Skid { sideways, .. } => {
            let gain = match (skid_axle, sideways) {
                (Some(1), _) => levels.skid_back,
                (_, true) => levels.skid_side,
                (_, false) => levels.skid_forward,
            };
            (gain, levels.skid_pitch)
        }
        SoundRef::RoadNoise(loop_value) => (levels.road.get(usize::from(loop_value)).copied().unwrap_or(0.0), 1.0),
        SoundRef::Wind => (levels.wind, levels.wind_pitch),
        SoundRef::Scrape(_) => (1.0, 1.0),
    }
}

/// The volume after the map's gain, never above the sample's own full scale.
fn loudness(volume: f32, gain: f32) -> f32 {
    (volume * gain).min(1.0)
}

/// `command` with its volume and pitch multiplied by the map's levels for its sound.
pub fn scale(levels: &Levels, command: SoundCommand) -> SoundCommand {
    match command {
        SoundCommand::Play { sound, volume, pitch } => {
            let (gain, ratio) = level_of(levels, sound, None);
            SoundCommand::Play { sound, volume: loudness(volume, gain), pitch: pitch * ratio }
        }
        SoundCommand::SetLoop { id, sound, volume, pitch } => {
            let axle = if let LoopId::Skid(axle) = id { Some(axle) } else { None };
            let (gain, ratio) = level_of(levels, sound, axle);
            SoundCommand::SetLoop { id, sound, volume: loudness(volume, gain), pitch: pitch * ratio }
        }
        SoundCommand::StopLoop(id) => SoundCommand::StopLoop(id),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn levels() -> Levels {
        Levels {
            clunk_up: 0.5,
            nitrous_pitch: 1.1,
            road: [0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9],
            ..Levels::unmixed()
        }
    }

    #[test]
    fn a_one_shot_takes_the_level_of_its_sound() {
        let play = SoundCommand::Play { sound: SoundRef::GearClunk { up: true }, volume: 0.8, pitch: 1.0 };
        assert_eq!(
            scale(&levels(), play),
            SoundCommand::Play { sound: SoundRef::GearClunk { up: true }, volume: 0.4, pitch: 1.0 }
        );
    }

    #[test]
    fn a_gain_above_one_stops_at_the_samples_full_scale() {
        let play = SoundCommand::Play { sound: SoundRef::GearClunk { up: false }, volume: 0.9, pitch: 1.0 };
        let l = Levels { clunk_down: 3.0, ..levels() };
        let SoundCommand::Play { volume, .. } = scale(&l, play) else { panic!() };
        assert_eq!(volume, 1.0);
    }

    #[test]
    fn a_road_loop_takes_the_level_of_its_surface_loop() {
        let l = SoundCommand::SetLoop { id: LoopId::Road(0), sound: SoundRef::RoadNoise(5), volume: 1.0, pitch: 1.0 };
        let SoundCommand::SetLoop { volume, .. } = scale(&levels(), l) else { panic!() };
        assert!((volume - 0.6).abs() < 1e-6);
    }

    #[test]
    fn pitch_levels_multiply_the_pitch() {
        let n = SoundCommand::SetLoop { id: LoopId::Nitrous, sound: SoundRef::Nitrous, volume: 1.0, pitch: 1.2 };
        let SoundCommand::SetLoop { pitch, .. } = scale(&levels(), n) else { panic!() };
        assert!((pitch - 1.32).abs() < 1e-6);
    }

    #[test]
    fn the_rear_axle_uses_the_back_level() {
        let sound = SoundRef::Skid { surface: 0, sideways: true };
        let mut l = levels();
        l.skid_back = 0.25;
        l.skid_side = 0.75;
        let rear = SoundCommand::SetLoop { id: LoopId::Skid(1), sound, volume: 1.0, pitch: 1.0 };
        let front = SoundCommand::SetLoop { id: LoopId::Skid(0), sound, volume: 1.0, pitch: 1.0 };
        let (SoundCommand::SetLoop { volume: r, .. }, SoundCommand::SetLoop { volume: f, .. }) =
            (scale(&l, rear), scale(&l, front))
        else {
            panic!()
        };
        assert_eq!((r, f), (0.25, 0.75));
    }

    #[test]
    fn stops_and_unknown_loop_values_pass_through() {
        assert_eq!(scale(&levels(), SoundCommand::StopLoop(LoopId::Wind)), SoundCommand::StopLoop(LoopId::Wind));
        let far =
            SoundCommand::SetLoop { id: LoopId::Road(0), sound: SoundRef::RoadNoise(40), volume: 1.0, pitch: 1.0 };
        let SoundCommand::SetLoop { volume, .. } = scale(&levels(), far) else { panic!() };
        assert_eq!(volume, 0.0);
    }
}
