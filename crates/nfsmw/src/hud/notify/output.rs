//! The HUD's sounds and pad rumbles (`docs/specs/custom-hud/feedback.md` sections 2 and 3): once a frame the cues and
//! rumbles the notifications raised (`signals.rs`) are played.
//!
//! - **Sound.** Each cue is a front-end bank trigger (`audio::menu::request_hud`), played in the HUD's volume group
//!   (`[custom_hud] hud_volume` times the master volume). No new sound asset: the triggers are the install's own, and
//!   which one each cue uses is the table of `docs/specs/custom-hud-audio-impl.md`. Cues are the Custom HUD's: they
//!   play only in the Custom style, while the HUD is shown.
//! - **Rumble.** A `GamepadRumbleRequest` to the driving player's pad, only with `[custom_hud] haptics` on (off by
//!   default) and a pad connected. A pad without force feedback is skipped by the pad backend.
//!
//! No warning depends on either: every cue and rumble comes with a notice or a chip on the screen.

use std::time::Duration;

use bevy_ecs::entity::Entity;
use bevy_ecs::message::Messages;
use bevy_ecs::prelude::*;
use bevy_ecs::system::NonSendMarker;
use bevy_input::gamepad::{GamepadRumbleIntensity, GamepadRumbleRequest};

use super::HudNotifications;
use super::catalog::{AudioCue, Haptic};
use crate::hud::HudState;
use crate::input::InputPresentation;
use crate::settings::hud::HudSettings;
use crate::settings::{Settings, UiStyle};

/// The front-end bank trigger each cue plays (`docs/specs/frontend-audio.md`: triggers below 23 are the common bank's,
/// the others the front-end bank's). Short menu sounds, apart from the pursuit music and the dispatch's speech. The
/// install has eight audible ones (0, 2, 4, 5, 7, 10, 11, 89) for ten cues, so two pairs share: the chase calming down
/// (contact lost, cooldown) and the bad ends (critical damage, busted). The table is in
/// `docs/specs/custom-hud-audio-impl.md`.
pub fn trigger(cue: AudioCue) -> i32 {
    match cue {
        // SOUND_UP / SOUND_DOWN: the lightest tick.
        AudioCue::Checkpoint => 0,
        // SOUND_LEFT / SOUND_RIGHT.
        AudioCue::Nitrous => 2,
        // SOUND_ACCEPT.
        AudioCue::Escaped => 4,
        // SOUND_BACK: a step back.
        AudioCue::MissedCheckpoint => 5,
        // SOUND_BUTTON0..5.
        AudioCue::CriticalDamage | AudioCue::Busted => 7,
        // SOUND_DLGBOX_IN: something new has come up.
        AudioCue::Escalation => 10,
        // SOUND_DLGBOX_OUT: the counterpart of the escalation.
        AudioCue::ContactLost | AudioCue::Cooldown => 11,
        // SOUND_MAIN_SUB: the front-end bank's own sample, apart from the common ones.
        AudioCue::Hazard => 89,
    }
}

/// The light rumble (`tap`): the weak motor at 0.3 for 80 ms.
pub const TAP: (GamepadRumbleIntensity, Duration) =
    (GamepadRumbleIntensity { strong_motor: 0.0, weak_motor: 0.3 }, Duration::from_millis(80));
/// The strong rumble (`thump`): the strong motor at 0.7 for 200 ms.
pub const THUMP: (GamepadRumbleIntensity, Duration) =
    (GamepadRumbleIntensity { strong_motor: 0.7, weak_motor: 0.0 }, Duration::from_millis(200));

/// The motors' intensities and the length of `haptic`.
pub fn rumble(haptic: Haptic) -> (GamepadRumbleIntensity, Duration) {
    match haptic {
        Haptic::Light => TAP,
        Haptic::Strong => THUMP,
    }
}

/// Whether the HUD's cues play: the Custom style, with the HUD on and shown.
pub fn plays_cues(style: UiStyle, hud_on: bool, state: &HudState) -> bool {
    style == UiStyle::Custom && hud_on && state.visible
}

/// The pad that rumbles: the driving player's `pad`, when `[custom_hud] haptics` is on.
pub fn rumble_pad(settings: &HudSettings, pad: Option<Entity>) -> Option<Entity> {
    if !settings.haptics {
        return None;
    }
    pad
}

/// Plays the frame's cues and rumbles. On the main thread, where the audio plugin takes the HUD's cue queue. The
/// cues and rumbles are taken every frame, so none waits to play late.
pub fn output(
    _main: NonSendMarker,
    settings: Res<Settings>,
    state: Res<HudState>,
    presentation: Option<Res<InputPresentation>>,
    rumbles: Option<ResMut<Messages<GamepadRumbleRequest>>>,
    mut notifications: ResMut<HudNotifications>,
) {
    let cues = notifications.take_cues();
    let haptics = notifications.take_haptics();
    if plays_cues(settings.ui_style, settings.hud, &state) {
        for cue in cues {
            crate::audio::menu::request_hud(trigger(cue));
        }
    }
    let pad = presentation.and_then(|presentation| presentation.active_pad);
    let Some(gamepad) = rumble_pad(&settings.hud_options, pad) else { return };
    let Some(mut rumbles) = rumbles else { return };
    for haptic in haptics {
        let (intensity, duration) = rumble(haptic);
        rumbles.write(GamepadRumbleRequest::Add { duration, intensity, gamepad });
    }
}
