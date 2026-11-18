//! A headless run of one NIS scene, for checks and the `nis-report` command: the scene is read and prepared as the world
//! does it (`loading.rs`), played to its end by the [`NisPlayer`] on a fixed 30 Hz clock with no window, no world and no
//! sound, and summed up as a [`SceneReport`]. The report says which real car each car channel would drive (by the same
//! rule as `car_world.rs`), how many frames played, which camera track plays, which events fire and which of them the
//! player acts on, and every fallback the world would take.
//!
//! Two lists close the report. `problems` are defects the install's data settles: a channel with no car, a car the data
//! names replaced by a stock pick, a camera other than the data's, a scene that stops before its last frame. `open` are
//! the remake's own choices where the data and the specs do not say what the original shows (a civilian channel taking
//! the nearest traffic car, the cop channels' cars, an overlay the player does not draw, events with no handler).

use std::collections::BTreeMap;

use game_install::GameDir;
use nfsmw_data::nis::{CameraLibrary, NisBundle};
use serde::Serialize;

use super::car_world::race_start_racer;
use super::cars::{CAR_SLOTS, Part, channel_of};
use super::cop_cars::{SlotKind, slot_kind};
use super::events::{NisEvent, NisEvents};
use super::kind::{NisKind, NisOutcome};
use super::loading::Loading;
use super::player::{NisPlayer, SCENE_FPS};
use crate::scenes::world::scene_space::Anchor;

/// The frame time of the run: one scene frame.
const FRAME: f32 = 1.0 / SCENE_FPS;
/// How long past the scene's end the event census keeps running, seconds, to find events the scene's end cuts off.
const CENSUS_TAIL: f32 = 60.0;

/// A racer a race's start scene may put on a car channel: its name, its car (`folder` or `folder:preset`) and whether
/// that car is a stock pick because the data names none.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Seat {
    pub name: String,
    pub car: Option<String>,
    pub stock: bool,
}

/// Who the scene's car channels can take. A race's start scene (`race_start`) puts the race's racers on `car1`..`car8`
/// by grid slot (`grid[i]` is the grid slot of `seats[i]`, the player first); any other scene puts the player on `car1`
/// and takes traffic cars for the rest. `listed` is how many racers the race's data names, the player included: a
/// channel the data has a racer for that no seat takes is a problem, one past them is the data's own gap.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Seating {
    pub race_start: bool,
    pub grid: Vec<usize>,
    pub seats: Vec<Seat>,
    pub listed: usize,
}

/// One car channel of the scene and the car it drives.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ChannelReport {
    /// `car1`..`car8`, `cop1`..`cop8`.
    pub channel: String,
    /// The scene time at which the channel's clips end, seconds.
    pub ends_at: Option<f32>,
    /// What the channel drives, in words; `none` when it stays off.
    pub binding: String,
}

/// The scene's camera.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CameraReport {
    /// The track the caller asked for (the race's `IntroCameraTrack`).
    pub asked: u32,
    /// The tracks of the scene's camera set in the install.
    pub tracks: usize,
    /// The track that plays, `None` when the slow orbit plays instead.
    pub plays: Option<String>,
    /// The playing track's length, seconds.
    pub seconds: Option<f32>,
}

/// One event of the scene's sequence as it fires.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct EventReport {
    /// Scene time, seconds.
    pub at: f32,
    pub event: String,
    /// The player acts on it (rather than logging it).
    pub handled: bool,
    /// It fires after the scene's last frame, so it never plays.
    pub after_end: bool,
}

/// The run of one scene.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SceneReport {
    pub scene: String,
    pub description: String,
    pub kind: String,
    pub scene_type: String,
    /// The header's `NumberOfCars`.
    pub number_of_cars: i32,
    /// The header's `VanishFrame`: the scene's length in frames.
    pub vanish_frame: i32,
    pub seconds: f32,
    /// The header's overlay name, empty when none.
    pub overlay: String,
    pub camera: CameraReport,
    pub channels: Vec<ChannelReport>,
    /// Character records in the scene, and the characters the player draws.
    pub character_records: usize,
    pub characters_drawn: usize,
    /// The engine-rev curves of `SOUND/NISREVDATA` were found.
    pub rev_curves: bool,
    /// The NIS stream take the scene plays, `None` when it plays none; `exact` false when it is a stand-in take.
    pub audio_take: Option<String>,
    pub audio_exact: bool,
    /// Real frames (30 a second) the player ran before the scene ended, and how it ended. They are more than the scene's
    /// own frames by the settle and by the camera's slow-motion keys.
    pub frames_played: u32,
    pub outcome: String,
    pub events: Vec<EventReport>,
    /// The events with no handler, by name, with their counts.
    pub unhandled: BTreeMap<String, u32>,
    pub problems: Vec<String>,
    pub open: Vec<String>,
}

/// Reads the scene `scene` from the install, plays it to its end with no window and reports it. `kind` and `track` are
/// what the world would pass; `seating` says who its car channels can take.
pub fn run(dir: &GameDir, scene: &str, kind: NisKind, track: u32, seating: &Seating) -> Result<SceneReport, String> {
    let bundle = NisBundle::load(dir, scene).map_err(|e| format!("{e:#}"))?;
    let library = CameraLibrary::load(dir).map_err(|e| format!("{e:#}"))?;
    let loading = Loading::start(dir, scene, Anchor::IDENTITY, kind, track)?;
    let ready = loading.prepare(loading.wait()?)?;
    let header = &bundle.scene;
    let seconds = ready.plan.clock.duration();
    let mut report = SceneReport {
        scene: bundle.name.clone(),
        description: header.description.clone(),
        kind: format!("{kind:?}"),
        scene_type: format!("{:?}", header.scene_type),
        number_of_cars: header.number_of_cars,
        vanish_frame: header.vanish_frame,
        seconds,
        overlay: header.overlay_name.clone(),
        camera: CameraReport {
            asked: track,
            tracks: library.nis(&bundle.name).or_else(|| library.fmv(&bundle.name)).map_or(0, <[_]>::len),
            plays: ready.plan.camera.as_ref().map(|track| track.name.clone()),
            seconds: ready.plan.camera.as_ref().map(|track| track.length),
        },
        channels: Vec::new(),
        character_records: header.characters.len(),
        characters_drawn: ready.characters.actor_count(),
        rev_curves: ready.revs.is_some(),
        audio_take: None,
        audio_exact: false,
        frames_played: 0,
        outcome: String::new(),
        events: Vec::new(),
        unhandled: BTreeMap::new(),
        problems: Vec::new(),
        open: Vec::new(),
    };
    report.channels = channels(&bundle, &ready.cars, kind, seating, &mut report);
    audio(dir, &bundle.name, &mut report);
    camera_findings(&mut report);
    let sequence = ready.sequence.clone();
    let mut player = NisPlayer::default();
    player.start_prepared(ready)?;
    let (frames, outcome) = play_out(&mut player, seconds);
    report.frames_played = frames;
    report.outcome = outcome.map_or_else(|| "never ended".to_owned(), |outcome| format!("{outcome:?}"));
    if outcome != Some(NisOutcome::Completed) || frames + 1 < header.vanish_frame.max(0) as u32 {
        report.problems.push(format!("the scene ran {frames} of its {} frames ({outcome:?})", header.vanish_frame));
    }
    if let Some(package) = sequence {
        report.events = census(package, &bundle.name, seconds, kind, &mut report.unhandled);
    }
    event_findings(&mut report);
    Ok(report)
}

/// The channels the bundle names and what each one drives. A channel with only one of its `_t` and `_q` clips does not
/// play, so it is a problem.
fn channels(
    bundle: &NisBundle,
    cars: &[super::cars::CarTrack],
    kind: NisKind,
    seating: &Seating,
    report: &mut SceneReport,
) -> Vec<ChannelReport> {
    let mut halves = [(false, false); CAR_SLOTS];
    for (_, bank) in &bundle.anim.banks {
        for animation in bank.iter() {
            let Some((slot, part)) = channel_of(&animation.name) else { continue };
            match part {
                Part::Translation => halves[slot].0 = true,
                Part::Rotation => halves[slot].1 = true,
            }
        }
    }
    let mut out = Vec::new();
    for (slot, half) in halves.iter().enumerate() {
        if *half == (false, false) {
            continue;
        }
        let channel = channel_name(slot);
        let track = cars.iter().find(|car| car.slot == slot);
        let Some(track) = track else {
            report.problems.push(format!("{channel}: its clips do not play (only one of _t and _q, or no root bone)"));
            out.push(ChannelReport { channel, ends_at: None, binding: "none".into() });
            continue;
        };
        let binding = bind(slot, kind, seating, &channel, report);
        out.push(ChannelReport { channel, ends_at: Some(track.end_time()), binding });
    }
    out
}

/// `car1`..`car8` for slots 0 to 7, `cop1`..`cop8` for 8 to 15.
fn channel_name(slot: usize) -> String {
    match slot < CAR_SLOTS / 2 {
        true => format!("car{}", slot + 1),
        false => format!("cop{}", slot - CAR_SLOTS / 2 + 1),
    }
}

/// What car channel `slot` drives, by the rules of `car_world.rs` (`link_nis_cars`), with the findings it gives.
fn bind(slot: usize, kind: NisKind, seating: &Seating, channel: &str, report: &mut SceneReport) -> String {
    match (seating.race_start, slot_kind(slot)) {
        (true, Some(SlotKind::Player | SlotKind::Civilian)) => {
            let Some(id) = race_start_racer(&seating.grid, slot) else {
                let finding = format!("{channel}: no racer stands on its grid slot, so the channel stays off");
                match slot < seating.listed {
                    true => report.problems.push(format!("{finding}, though the race's data names one")),
                    false => report.open.push(format!("{finding}; the race's data names {} racers", seating.listed)),
                }
                return "none".into();
            };
            let Some(seat) = seating.seats.get(id) else { return "none".into() };
            let car = seat.car.as_deref().unwrap_or("no car");
            if seat.stock {
                report
                    .problems
                    .push(format!("{channel}: {} drives a stock pick ({car}); the data names no car", seat.name));
            }
            if seat.car.is_none() {
                report.problems.push(format!("{channel}: {} has no car", seat.name));
            }
            match id {
                0 => format!("player ({car})"),
                _ => format!("racer {id} {} ({car})", seat.name),
            }
        }
        (_, Some(SlotKind::Player)) => "player".into(),
        (_, Some(SlotKind::Civilian)) => {
            report.open.push(format!("{channel}: takes the nearest civilian traffic car (the remake's choice)"));
            "nearest traffic car".into()
        }
        (_, Some(SlotKind::Cop)) => {
            let rule = match super::cop_cars::cop_source(kind) {
                super::cop_cars::CopSource::BorrowFirst => "a chase car, else a spawned copmidsize",
                super::cop_cars::CopSource::SpawnFirst => "a spawned copmidsize, else a chase car",
            };
            report.open.push(format!("{channel}: takes {rule}; the scene's special car list is not read"));
            format!("police car: {rule}")
        }
        (_, None) => "none".into(),
    }
}

/// The scene's NIS stream take.
fn audio(dir: &GameDir, scene: &str, report: &mut SceneReport) {
    match crate::audio::nis::take_of(dir, scene) {
        Ok(Some(take)) => {
            report.audio_take = Some(format!("bank {} take {}", take.bank, take.take));
            report.audio_exact = take.exact;
            if !take.exact {
                report.open.push("the NIS stream plays the first take of its branch, not a take of its own".into());
            }
        }
        Ok(None) => report.open.push("the scene plays no NIS stream".into()),
        Err(error) => report.problems.push(format!("the NIS stream cannot be read: {error}")),
    }
}

/// The camera findings: the slow orbit when the install has no set, another track than the one asked for.
fn camera_findings(report: &mut SceneReport) {
    let camera = &report.camera;
    let asked = format!("Track {}", camera.asked);
    match (&camera.plays, camera.tracks) {
        (None, 0) => report.open.push("the install has no camera set for the scene: a slow orbit plays".into()),
        (None, _) => report.problems.push("the scene's camera set does not read: a slow orbit plays".into()),
        (Some(plays), _) if *plays != asked => {
            report.open.push(format!("the race asks for {asked}, which the set lacks: {plays} plays"));
        }
        _ => {}
    }
    if let Some(length) = camera.seconds
        && (length - report.seconds).abs() > 0.5
    {
        report.open.push(format!("the camera track lasts {length:.2} s against the scene's {:.2} s", report.seconds));
    }
    if !report.overlay.is_empty() {
        report.open.push(format!("the scene's overlay {} is not drawn (nis-player-2.md section 11)", report.overlay));
    }
}

/// Plays the started scene on whole frames until it ends; at most twice its length.
fn play_out(player: &mut NisPlayer, seconds: f32) -> (u32, Option<NisOutcome>) {
    let limit = ((seconds * 2.0 + 1.0) * SCENE_FPS) as u32;
    let mut frames = 0;
    while frames < limit {
        player.advance(FRAME);
        frames += 1;
        if let Some(outcome) = player.take_outcome() {
            return (frames, Some(outcome));
        }
    }
    (frames, None)
}

/// The scene's events as they fire on the scene's clock, run on past the end to find what the end cuts off.
fn census(
    package: std::sync::Arc<blackbox_sequence::Package>,
    scene: &str,
    seconds: f32,
    kind: NisKind,
    unhandled: &mut BTreeMap<String, u32>,
) -> Vec<EventReport> {
    let Some(mut events) = NisEvents::start(package, scene) else { return Vec::new() };
    let mut out = Vec::new();
    let mut at = 0.0;
    let mut step = 0.0;
    while at <= seconds + CENSUS_TAIL {
        let after_end = at > seconds + FRAME / 2.0;
        for event in events.advance(step) {
            let (event, handled) = describe(&event, kind);
            out.push(EventReport { at, event, handled, after_end });
        }
        for id in events.take_unhandled() {
            let name = event_name(id);
            *unhandled.entry(name.clone()).or_default() += 1;
            out.push(EventReport { at, event: name, handled: false, after_end });
        }
        step = FRAME;
        at += FRAME;
    }
    out
}

/// An event's name with its values, and whether the player acts on it (`handlers.rs`: `dispatch`).
fn describe(event: &NisEvent, kind: NisKind) -> (String, bool) {
    match event {
        NisEvent::Car { event, .. } => (format!("{event:?}"), true),
        NisEvent::CellCall(id) => (format!("ECellCall {id}"), true),
        NisEvent::ScreenFlash { length } => (format!("ENISScreenFlash {length}"), true),
        NisEvent::PlayEffect { name, .. } => (format!("ENISPlayEffect {name}"), true),
        NisEvent::Overlay { package, message } => {
            let busted = kind == NisKind::Arrest && package.eq_ignore_ascii_case(crate::hud::OVERLAY_PACKAGE);
            let fade = package.eq_ignore_ascii_case(super::handlers::BLACK_FADE_PACKAGE)
                && super::handlers::black_fade(message).is_some();
            let drawn = busted || fade;
            (format!("ENISOverlayMessage {package} {message}"), drawn)
        }
        other => (format!("{other:?}"), false),
    }
}

/// The findings of the events: those with no handler, and those that fire after the end.
fn event_findings(report: &mut SceneReport) {
    for (name, count) in &report.unhandled {
        report.open.push(format!("{name} x{count}: no handler"));
    }
    let mut logged: BTreeMap<String, u32> = BTreeMap::new();
    for event in report.events.iter().filter(|event| !event.handled && !report.unhandled.contains_key(&event.event)) {
        let name = event.event.split([' ', '{', '(']).next().unwrap_or_default().to_owned();
        *logged.entry(name).or_default() += 1;
    }
    for (name, count) in logged {
        report.open.push(format!("{name} x{count}: decoded, but the player does not act on it"));
    }
    let cut: Vec<String> = report
        .events
        .iter()
        .filter(|event| event.after_end)
        .map(|event| format!("{} at {:.2} s", event.event, event.at))
        .collect();
    if !cut.is_empty() {
        report.problems.push(format!("events after the scene's last frame never play: {}", cut.join(", ")));
    }
}

/// The class name of an event type hash (`docs/specs/nis-events.md` sections 2 and 3), or the hash.
fn event_name(id: u32) -> String {
    const NAMES: [(u32, &str); 26] = [
        (0x8DA0_5B3D, "ENISAeroDynamics"),
        (0xA01A_259B, "ENISBrakelock"),
        (0x991D_D250, "ENISCarDamageReset"),
        (0x3F56_A04E, "ENISCopCarDoors"),
        (0x257A_10A8, "ENISDetach"),
        (0x49D3_4FF1, "ENISFakeFar"),
        (0xFC38_2D92, "ENISMotionBlur"),
        (0x00EB_198B, "ENISNeutralRev"),
        (0xEB34_8F62, "ENISNitro"),
        (0xA87E_176D, "ENISNukeSmack"),
        (0x86D3_68CB, "ENISPixelate"),
        (0x6A46_5054, "ENISRain"),
        (0xCF01_B27E, "ENISReattach"),
        (0xB9A0_2FA8, "ENISRoadNoise"),
        (0x7282_975D, "ENISSteering"),
        (0xC058_2354, "ENISStopEffects"),
        (0xF699_BF04, "ENISTimeOfDay"),
        (0x41D6_52BB, "ENISVisualLook"),
        (0x5096_49B1, "ENISWolrdGeometry"),
        (0x62F1_0875, "ENISWorldAnimTrigger"),
        (0x22AA_36CE, "ENISConstraint"),
        (0x74E9_6511, "ENISDetail"),
        (0x2096_C371, "ECameraPhotoFinish"),
        (0xA67E_8286, "EDDaySpeech"),
        (0xC1AF_4FDD, "EPlayObjectEffect"),
        (0x3338_EFB8, "EStopObjectEffect"),
    ];
    NAMES.iter().find(|(hash, _)| *hash == id).map_or_else(|| format!("{id:08X}"), |(_, name)| (*name).to_owned())
}

#[cfg(test)]
#[path = "report_tests.rs"]
mod tests;
