//! One ownership decision for every effect command; adapters do not delete commands.
use super::CarAudio;
use crate::audio::{Audio, mixer::scale_command, trace::Record};
use blackbox_carsound::{LoopId, SoundCommand, SoundRef};

pub(super) fn family(sound: SoundRef) -> &'static str {
    match sound {
        SoundRef::Nitrous => "FX_NITROUS",
        SoundRef::Purge => "FX_PURGE",
        SoundRef::Sweetener(_) => "CAR_SWTN",
        SoundRef::GearClunk { .. } | SoundRef::BrakeMash => "FX_SHIFTING_01",
        SoundRef::TurboSpool | SoundRef::TurboBlowoff(_) => "FX_TURBO_01",
        SoundRef::ReverseWhine => "CAR_WHINE",
        SoundRef::Skid { .. } => "FX_SKID",
        SoundRef::RoadNoise(_) => "FX_ROADNOISE",
        SoundRef::Wind => "FX_WIND",
        SoundRef::Scrape(_) => "FX_Scrape",
    }
}

pub(in crate::audio) fn command_family(command: SoundCommand) -> &'static str {
    match command {
        SoundCommand::Play { sound, .. } | SoundCommand::SetLoop { sound, .. } => family(sound),
        SoundCommand::StopLoop(id) => family(loop_sound(id)),
    }
}

fn loop_sound(id: LoopId) -> SoundRef {
    match id {
        LoopId::Reverse => SoundRef::ReverseWhine,
        LoopId::Turbo => SoundRef::TurboSpool,
        LoopId::Nitrous => SoundRef::Nitrous,
        LoopId::Skid(_) => SoundRef::Skid { surface: 0, sideways: false },
        LoopId::Road(_) => SoundRef::RoadNoise(0),
        LoopId::Wind => SoundRef::Wind,
        LoopId::Scrape => SoundRef::Scrape(blackbox_carsound::ScrapeKind::Ground),
    }
}

fn owned(car: &CarAudio, command: SoundCommand, tyres: bool, power_event: bool) -> bool {
    let (id, sound) = match command {
        SoundCommand::Play { sound, .. } => return power_event && car.power.owns(sound),
        SoundCommand::SetLoop { id, sound, .. } => (id, sound),
        SoundCommand::StopLoop(id) => (id, loop_sound(id)),
    };
    if matches!(id, LoopId::Skid(_)) {
        return tyres;
    }
    if car.ambient.owns_loop(id) {
        return true;
    }
    car.power.owns(sound)
}

pub(super) fn dispatch(audio: &mut Audio, car: &mut CarAudio, command: SoundCommand, tyres: bool, power_event: bool) {
    if !owned(car, command, tyres, power_event) {
        audio.apply(car, scale_command(&car.levels, command));
        return;
    }
    if audio.trace.enabled() {
        audio.trace.record(Record {
            event: "router_owned",
            class: command_family(command),
            logical_ms: audio.trace.now_ms(),
            success: Some(true),
            detail: &format!("{command:?}"),
            ..Default::default()
        });
    }
    let id = match command {
        SoundCommand::SetLoop { id, .. } | SoundCommand::StopLoop(id) => Some(id),
        _ => None,
    };
    if let Some(id) = id
        && let Some((_, mut handle)) = car.loops.remove(&id)
    {
        handle.stop(kira::Tween::default());
        audio.trace.note("fallback_retire", &format!("{id:?}"));
    }
}
