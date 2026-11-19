//! Menu event routing and authored bank playback. Spec: `frontend-audio.md`. The HUD's cues reuse the same triggers
//! ([`request_hud`]), played in the HUD's volume group instead of the effects group
//! (`docs/specs/custom-hud-audio-impl.md`).

use std::cell::{Cell, RefCell};
use std::collections::HashSet;

use blackbox_feng::ids::*;

use super::aems::{EffectModule, EffectSampler};
use super::{Audio, Group};

mod cue_log;
#[cfg(test)]
mod inventory_tests;
mod messages;
#[cfg(test)]
mod movie_gate_tests;
mod photo;
#[cfg(test)]
mod tests;
pub use photo::{request_photo_finish, request_photo_transition};

use blackbox_feng::PackageId;
use blackbox_feng::runtime::Outgoing;

use messages::stock_trigger;

pub(crate) use cue_log::CueNote;

const LAST_COMMON_BUTTON: u32 = PAD_BUTTON0 + 5;

enum Request {
    /// A cue to start; the note says where the front end decided it, `None` for a direct request.
    Play(i32, Option<CueNote>),
    /// A front-end cue a gate suppressed, kept for the audio trace.
    Suppressed(CueNote),
    StopRapSheet(i32),
}

thread_local! {
    static QUEUE: RefCell<Vec<Request>> = const { RefCell::new(Vec::new()) };
    static HUD_QUEUE: RefCell<Vec<i32>> = const { RefCell::new(Vec::new()) };
    static COMMON_PREVIEW: Cell<Option<f32>> = const { Cell::new(None) };
}

/// An audio slider supplies the level for the next common menu cue.
pub(crate) fn preview_common_level(level: f32) {
    COMMON_PREVIEW.set(Some(level));
}

fn class_gain(id: i32, group: Group) -> i32 {
    if group != Group::Sfx {
        return 32767;
    }
    // The FEHUD channel adds its preset and quantized master control to the -10 dB base.
    let preset = if id < 23 { -422 } else { -533 };
    let full_level = blackbox_mixmap::db_from_q15(blackbox_mixmap::UNITY);
    let gain = blackbox_mixmap::q15_from_db(-1000 + preset + full_level);
    if id >= 23 {
        return gain;
    }
    let Some(preview) = COMMON_PREVIEW.take() else { return gain };
    (gain * (preview.clamp(0.0, 1.0) * 32767.0) as i32) >> 15
}

fn trigger(message: u32) -> Option<i32> {
    Some(match message {
        PAD_UP | PAD_DOWN | PAD_UPLEFT | PAD_UPRIGHT | PAD_DOWNLEFT | PAD_DOWNRIGHT => 0,
        PAD_LEFT | PAD_RIGHT => 2,
        PAD_ACCEPT | BUTTON_PRESSED => 4,
        PAD_BACK => 5,
        PAD_START => 6,
        PAD_BUTTON0..=LAST_COMMON_BUTTON => 7,
        _ => return stock_trigger(message),
    })
}

/// Requests a menu effect; unknown messages and button releases stay silent.
pub fn request(message: u32) {
    if let Some(id) = trigger(message) {
        request_trigger(id);
    }
}

/// Requests a trigger directly, for a sound the original's screen code plays by id (the original's routine: ids below 23 are
/// common samples, the rest front-end samples, the same split as [`target`]).
pub fn request_trigger(id: i32) {
    QUEUE.with(|queue| queue.borrow_mut().push(Request::Play(id, None)));
}

/// Requests a front-end cue the screens let through, with its decision for the audio trace.
pub(crate) fn request_cue(id: i32, note: CueNote) {
    QUEUE.with(|queue| queue.borrow_mut().push(Request::Play(id, Some(note))));
}

/// Records a front-end cue a gate suppressed. Only the audio trace reads it; a run without audio keeps a bounded queue.
pub(crate) fn note_suppressed(note: CueNote) {
    QUEUE.with(|queue| {
        let mut queue = queue.borrow_mut();
        if queue.len() < 4096 {
            queue.push(Request::Suppressed(note));
        }
    });
}

/// Stops the held rap-sheet cue if it matches this login page.
pub fn stop_rap_sheet(id: i32) {
    QUEUE.with(|queue| queue.borrow_mut().push(Request::StopRapSheet(id)));
}

/// Tracks authored cues and the equivalent host input feedback for this frame.
pub(crate) struct FrameSounds {
    authored: HashSet<(PackageId, i32)>,
    input: HashSet<(PackageId, i32)>,
}

impl FrameSounds {
    pub(crate) fn new(outgoing: &[Outgoing]) -> Self {
        let authored = outgoing
            .iter()
            .filter_map(|event| match event {
                Outgoing::Sound { package, message, authored: true, .. } => Some((*package, stock_trigger(*message)?)),
                _ => None,
            })
            .collect();
        Self { authored, input: HashSet::new() }
    }

    /// An outer None marks redundant input; an inner None leaves the decision to the screen.
    #[cfg(test)]
    pub(crate) fn candidate(&mut self, package: PackageId, message: u32, authored: bool) -> Option<Option<i32>> {
        self.decide(package, message, authored).ok()
    }

    /// The candidate trigger, or why this frame drops the message before any screen sees it.
    pub(crate) fn decide(
        &mut self,
        package: PackageId,
        message: u32,
        authored: bool,
    ) -> Result<Option<i32>, &'static str> {
        if authored {
            return Ok(stock_trigger(message));
        }
        let id = trigger(message).ok_or("unmapped input")?;
        if self.authored.contains(&(package, id)) {
            return Err("frame: authored post has this cue");
        }
        if !self.input.insert((package, id)) {
            return Err("frame: input already gave this cue");
        }
        Ok(Some(id))
    }
}

fn take_requests() -> Vec<Request> {
    QUEUE.with(|queue| std::mem::take(&mut *queue.borrow_mut()))
}

#[cfg(test)]
pub(crate) fn take() -> Vec<i32> {
    take_requests()
        .into_iter()
        .filter_map(|request| match request {
            Request::Play(id, _) => Some(id),
            Request::Suppressed(_) | Request::StopRapSheet(_) => None,
        })
        .collect()
}

/// Requests trigger `id` (a front-end bank trigger, as [`trigger`] gives them) as one of the HUD's cues: it plays like
/// a menu sound, in the HUD's volume group. Call it on the main thread, where the audio plugin takes the queue.
pub fn request_hud(id: i32) {
    HUD_QUEUE.with(|queue| queue.borrow_mut().push(id));
}

pub(crate) fn take_hud() -> Vec<i32> {
    HUD_QUEUE.with(|queue| std::mem::take(&mut *queue.borrow_mut()))
}

fn target(id: i32) -> (&'static str, &'static str, i32) {
    if id < 23 {
        return ("GLOBAL/FE_COMMON_MB.abk", "PlayCommonSample", id);
    }
    let class = match id {
        103..=109 | 112..=118 => "PlayFrontEndSample_RS",
        _ => "PlayFrontEndSample",
    };
    ("FE/FE_MB.abk", class, id - 23)
}

struct Running {
    module: EffectModule,
    words: Vec<i32>,
    age: f32,
    released: bool,
    group: Group,
    /// Release only once the voices have run out: a graph whose destroy node stops its voices (`FX_Camera`).
    hold: bool,
    /// Rap sheet owns one persistent module slot.
    rap_sheet: bool,
}

/// Sound lifetimes survive screen changes and end independently of the menu stack.
#[derive(Default)]
pub(super) struct MenuPlayer {
    active: Vec<Running>,
    unavailable: HashSet<i32>,
    seed: u32,
}

impl MenuPlayer {
    #[cfg(test)]
    fn start(&mut self, audio: &mut Audio, id: i32) {
        self.start_in(audio, id, Group::Sfx);
    }

    /// Starts trigger `id` in the mixer group `group` and says what became of it, for the audio trace.
    fn start_in(&mut self, audio: &mut Audio, id: i32, group: Group) -> String {
        if !(0..160).contains(&id) {
            return "dropped: trigger out of range".into();
        }
        if audio.output.is_none() {
            return "dropped: no output".into();
        }
        let (bank, class, index) = target(id);
        // A boot or story movie mutes the front-end effects (FMV sound state, `docs/specs/frontend-audio.md`).
        if audio.movie_active && group == Group::Sfx {
            return "dropped: movie playing".into();
        }
        let rap_sheet = class == "PlayFrontEndSample_RS";
        if rap_sheet {
            self.active.retain(|running| !running.rap_sheet);
        }
        let gain = class_gain(id, group);
        let words = [index, gain, 4096, 0];
        let group_name = match group {
            Group::Sfx => "Sfx",
            Group::World => "World",
            Group::Hud => "Hud",
            Group::Music => "Music",
            Group::Engine => "Engine",
        };
        if self.unavailable.contains(&id) {
            audio.trace.record(super::trace::Record {
                event: "menu_trigger",
                trigger: Some(id),
                group: Some(group_name),
                bank,
                class,
                params: &words,
                success: Some(false),
                detail: "cached unavailable",
                logical_ms: audio.trace.now_ms(),
                ..Default::default()
            });
            return "failed: cached unavailable".into();
        }
        self.seed = self.seed.wrapping_add(1);
        match EffectModule::load(audio, bank, class, self.seed) {
            Ok(module) => {
                let instance = module.trace_instance();
                audio.trace.record(super::trace::Record {
                    event: "menu_trigger",
                    instance,
                    trigger: Some(id),
                    group: Some(group_name),
                    bank,
                    class,
                    params: &words,
                    success: Some(true),
                    logical_ms: audio.trace.now_ms(),
                    ..Default::default()
                });
                self.active.push(Running {
                    module,
                    words: words.to_vec(),
                    age: 0.0,
                    released: false,
                    group,
                    hold: false,
                    rap_sheet,
                });
                format!("started instance={instance}")
            }
            Err(error) => {
                audio.trace.record(super::trace::Record {
                    event: "menu_trigger",
                    trigger: Some(id),
                    group: Some(group_name),
                    bank,
                    class,
                    params: &words,
                    success: Some(false),
                    detail: &error,
                    logical_ms: audio.trace.now_ms(),
                    ..Default::default()
                });
                self.unavailable.insert(id);
                log::warn!("menu sound {id}: {error}");
                format!("failed: {error}")
            }
        }
    }

    fn update(&mut self, audio: &mut Audio, dt: f32) {
        if !dt.is_finite() || dt <= 0.0 {
            return;
        }
        self.active.retain_mut(|running| {
            running.age += dt;
            let usable = running.module.update(&mut EffectSampler(audio, running.group), dt, &running.words);
            // The graph must see its class data once before its short-lived Csis owner is released.
            let free = !running.hold || running.module.silent();
            if !running.rap_sheet && !running.released && running.age >= 1.0 / 60.0 && free {
                running.module.release();
                running.released = true;
            }
            usable && !running.module.finished() && (running.rap_sheet || running.age < 10.0)
        });
    }

    pub(super) fn service(&mut self, audio: &mut Audio, dt: f32) {
        if photo::take_transition() {
            photo::start_transition(audio);
        }
        if photo::take()
            && let Err(error) = self.start_photo(audio)
        {
            log::warn!("photo camera sound unavailable: {error}");
        }
        for request in take_requests() {
            match request {
                Request::Play(id, note) => {
                    let voice = self.start_in(audio, id, Group::Sfx);
                    cue_log::played(audio, id, note.as_ref(), &voice);
                }
                Request::Suppressed(note) => cue_log::suppressed(audio, &note),
                Request::StopRapSheet(id) => self.active.retain(|cue| !cue.rap_sheet || cue.words[0] + 23 != id),
            }
        }
        for id in take_hud() {
            self.start_in(audio, id, Group::Hud);
        }
        self.update(audio, dt);
    }
}

impl Audio {
    pub(crate) fn update_menu(&mut self, dt: f32) {
        let mut player = std::mem::take(&mut self.menu);
        player.service(self, dt);
        self.menu = player;
    }
}
