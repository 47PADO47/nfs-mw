//! What a NIS scene's events do: the flash and the overlay it puts over the frame, the phone call it makes, and the named
//! effect it plays. The events are decoded in `events.rs`. Each event is handled here or logged, as its doc says.
//! Spec: `docs/specs/nis-events.md` sections 2 and 6.

use blackbox_attrib::vlt_hash;
use blackbox_feng::fe_hash_upper;
use blackbox_particles::Frame;
use glam::{Mat4, Vec3};
use nfsmw_data::world::sequences::ObjectEffect;

use super::events::NisEvent;
use crate::scenes::world::WorldScene;
use crate::scenes::world::prop_effects::{Command, Source};

/// The particle source a NIS scene's named effects play under. It is not a sequence object's id.
const NIS_EFFECT_SOURCE: u32 = 0x4E49_5345;

/// The overlay package whose messages fade the screen to black and back (`EndingNis03` sends it `15_OUT`).
pub const BLACK_FADE_PACKAGE: &str = "BLACK_FADE_OUT.fng";

/// What a NIS scene puts over the frame: a full-screen flash, a black fade, and the last overlay message it sent. They
/// end with the scene, except a fade that reached black: the screen stays black ([`NisScreen::take_held_black`]).
#[derive(Debug, Default, Clone, PartialEq)]
pub struct NisScreen {
    flash: Option<Flash>,
    overlay: Option<Overlay>,
    fade: Option<Fade>,
    /// The scene ended black, and the world has not taken the black over yet.
    held_black: bool,
}

/// A fade of the black panel of [`BLACK_FADE_PACKAGE`]: its opacity goes from `from` to `to` over `length` seconds.
#[derive(Debug, Clone, PartialEq)]
struct Fade {
    from: f32,
    to: f32,
    length: f32,
    elapsed: f32,
}

impl Fade {
    fn alpha(&self) -> f32 {
        let done = (self.elapsed / self.length).clamp(0.0, 1.0);
        self.from + (self.to - self.from) * done
    }
}

/// The fade a message of [`BLACK_FADE_PACKAGE`] starts: the black panel's opacity at the start and at the end, and the
/// seconds it takes. From install data: each message starts one script of the package's panel, whose colour track runs
/// the alpha from 0 to 255 (`_OUT`) or from 255 to 0 (`_IN`) over 300 (`15_`) or 600 (`30_`) ticks, at 960 ticks a
/// second (`nfsmw dump-screen BLACK_FADE_OUT.fng`). `None` for any other message.
pub fn black_fade(message: &str) -> Option<(f32, f32, f32)> {
    let (from, to, ticks) = match message.to_ascii_uppercase().as_str() {
        "15_OUT" => (0.0, 1.0, 300.0),
        "15_IN" => (1.0, 0.0, 300.0),
        "30_OUT" => (0.0, 1.0, 600.0),
        "30_IN" => (1.0, 0.0, 600.0),
        _ => return None,
    };
    Some((from, to, ticks / blackbox_feng::runtime::TICKS_PER_SECOND))
}

#[derive(Debug, Clone, PartialEq)]
struct Flash {
    length: f32,
    elapsed: f32,
}

#[derive(Debug, Clone, PartialEq)]
struct Overlay {
    package: String,
    message: String,
    /// Counts the scene's overlay events, so the HUD posts each one to its package once.
    serial: u32,
}

impl NisScreen {
    /// `ENISScreenFlash`: a white flash that fades over `length` seconds. A length that is not positive shows nothing.
    pub fn start_flash(&mut self, length: f32) {
        if length.is_nan() || length <= 0.0 {
            return;
        }
        self.flash = Some(Flash { length, elapsed: 0.0 });
    }

    /// `ENISOverlayMessage`: the message the scene sends to an overlay package, kept until the next overlay event or the
    /// end of the scene. The event shows nothing itself: the original only queues the message to the package, which the
    /// arrest camera has put up (`docs/specs/busted-sequence.md` section 6.1).
    pub fn show_overlay(&mut self, package: String, message: String) {
        let fade = black_fade(&message).filter(|_| package.eq_ignore_ascii_case(BLACK_FADE_PACKAGE));
        if let Some((from, to, length)) = fade {
            self.fade = Some(Fade { from, to, length, elapsed: 0.0 });
        }
        let serial = self.overlay.as_ref().map_or(0, |overlay| overlay.serial.wrapping_add(1));
        self.overlay = Some(Overlay { package, message, serial });
    }

    /// Runs `dt` seconds of scene time: the flash fades, and it ends when its length has passed.
    pub fn advance(&mut self, dt: f32) {
        if let Some(fade) = self.fade.as_mut() {
            fade.elapsed += dt.max(0.0);
        }
        let Some(flash) = self.flash.as_mut() else { return };
        flash.elapsed += dt.max(0.0);
        if flash.elapsed >= flash.length {
            self.flash = None;
        }
    }

    /// The flash's white now: 1 at its start, falling to 0 at its end. `None` when no flash shows.
    pub fn flash_alpha(&self) -> Option<f32> {
        let flash = self.flash.as_ref()?;
        Some((1.0 - flash.elapsed / flash.length).clamp(0.0, 1.0))
    }

    /// The last overlay event, as its package and its message. `None` before the scene sends one.
    #[cfg(test)]
    pub fn overlay(&self) -> Option<(&str, &str)> {
        self.overlay.as_ref().map(|overlay| (overlay.package.as_str(), overlay.message.as_str()))
    }

    /// The last overlay event when it went to `package` (any case): the message's FEng hash and the event's serial, for
    /// the HUD to post to its copy of the package once per event.
    pub fn overlay_cue(&self, package: &str) -> Option<(u32, u32)> {
        let overlay = self.overlay.as_ref().filter(|overlay| overlay.package.eq_ignore_ascii_case(package))?;
        Some((fe_hash_upper(&overlay.message), overlay.serial))
    }

    /// The black fade's opacity now, 0 when no fade runs.
    pub fn black_alpha(&self) -> f32 {
        self.fade.as_ref().map_or(0.0, Fade::alpha)
    }

    /// Whether a flash or a black fade shows, so [`Self::draw`] puts something over the frame. The other overlay packages
    /// are the HUD's.
    pub fn is_showing(&self) -> bool {
        self.flash.is_some() || self.black_alpha() > 0.0
    }

    /// Clears the flash, the fade and the overlay: a new scene starts.
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    /// The scene has ended: everything clears, and a fade that had reached black is held for the world to take.
    pub fn end(&mut self) {
        let black = self.black_alpha() >= 1.0;
        *self = Self { held_black: black, ..Self::default() };
    }

    /// Whether the scene that ended left the screen black, once.
    pub fn take_held_black(&mut self) -> bool {
        std::mem::take(&mut self.held_black)
    }

    /// Draws the flash over the frame.
    pub fn draw(&self, ctx: &egui::Context) {
        if !self.is_showing() {
            return;
        }
        let screen = ctx.content_rect();
        let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::new("nis_screen")));
        let black = self.black_alpha();
        if black > 0.0 {
            painter.rect_filled(screen, 0.0, egui::Color32::from_black_alpha((black * 255.0).round() as u8));
        }
        if let Some(alpha) = self.flash_alpha() {
            painter.rect_filled(screen, 0.0, egui::Color32::from_white_alpha((alpha * 255.0) as u8));
        }
    }
}

/// What the world does for an event, once the screen has taken its part.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum Request {
    /// Queue the phone call or message `id` through the career phone.
    Call(u32),
    /// Play a named effect at a point of the scene.
    Effect(EffectRequest),
}

/// A named effect at a point of the scene (scene space), with its magnitude vector.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct EffectRequest {
    pub name: String,
    pub position: [f32; 3],
    pub magnitude: [f32; 3],
}

/// Applies one NIS event to the screen and returns what the world must still do. The events the player does not act
/// on yet (characters, freezes, lights, camera shake) are logged and return nothing.
pub(super) fn dispatch(event: NisEvent, screen: &mut NisScreen) -> Option<Request> {
    match event {
        NisEvent::CellCall(id) => Some(Request::Call(id)),
        NisEvent::ScreenFlash { length } => {
            screen.start_flash(length);
            None
        }
        NisEvent::Overlay { package, message } => {
            log::debug!("NIS overlay {package}: {message}");
            screen.show_overlay(package, message);
            None
        }
        NisEvent::CarShake { shake, rate, ramp, time } => {
            log::debug!(
                "NIS car shake {shake} at rate {rate}, ramp {ramp}, for {time} s: the camera does not shake yet"
            );
            None
        }
        NisEvent::HideCharacter { name, show } => {
            log::debug!("NIS character {name}: show {show}; characters are not drawn yet");
            None
        }
        NisEvent::Freeze(on) => {
            log::debug!("NIS freeze {on}: not applied yet");
            None
        }
        NisEvent::Lights { cop, head } => {
            log::debug!("NIS lights: cop {cop}, head {head}: not applied to the cars yet");
            None
        }
        NisEvent::CopLights(on) => {
            log::debug!("NIS cop lights {on}: not applied to the cars yet");
            None
        }
        NisEvent::PlayEffect { name, position, magnitude } => {
            Some(Request::Effect(EffectRequest { name, position, magnitude }))
        }
        NisEvent::Car { .. } | NisEvent::Other(_) => None,
    }
}

impl WorldScene {
    /// Applies the events a NIS scene fired this frame, in order.
    pub(in crate::scenes::world) fn apply_nis_events(&mut self, events: Vec<NisEvent>) {
        for event in events {
            match dispatch(event, self.nis.screen_mut()) {
                Some(Request::Call(id)) => self.career.phone.call(id),
                Some(Request::Effect(effect)) => self.play_nis_effect(effect),
                None => {}
            }
        }
    }

    /// Plays a NIS scene's named effect at its point, under the scene: it stops when the scene ends, because its owner
    /// is the scene's.
    fn play_nis_effect(&mut self, effect: EffectRequest) {
        if effect.name.is_empty() {
            log::debug!("NIS effect with no name is skipped");
            return;
        }
        let (Some(anchor), Some(owner)) = (self.nis.anchor(), self.nis.effect_owner()) else { return };
        let at = anchor.to_render(effect.position);
        log::debug!("NIS effect {} at {at:?}, magnitude {:?}", effect.name, effect.magnitude);
        let source = Source {
            id: NIS_EFFECT_SOURCE,
            owner,
            effect: ObjectEffect {
                name_tag: 0,
                effect: vlt_hash(&effect.name),
                position: 0,
                intensity: 1.0,
                instance: 0,
                tracking: false,
                allow_hidden_spawn: true,
            },
            frame: Frame { to_world: Mat4::from_translation(at), inherit_velocity: Vec3::ZERO, intensity: 1.0 },
        };
        self.object_fx.request([Command::Play(source)]);
    }
}

#[cfg(test)]
#[path = "handlers_tests.rs"]
mod tests;
