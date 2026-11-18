//! The event sequence of a NIS scene: its CARP engine, started on the `START` stimulus and run on the scene's clock.
//! The events that fire are decoded into [`NisEvent`]s, and `handlers.rs` says what each one does. Spec:
//! `docs/specs/nis-events.md` sections 1 and 6, and `docs/specs/nis-events-2.md` for the sample timelines.

use std::collections::BTreeSet;
use std::sync::Arc;

use blackbox_sequence::{Event, Package, Player, QueueMode};
use nfsmw_data::world::sequences::TAGS;

use super::car_events::CarEvent;

/// The stimulus that starts a scene's events: the word `START` (`docs/specs/nis-events.md` section 1.2).
pub const START: u32 = 0xA6A5_6EA1;

/// An event a NIS scene fires, with the parameters the player reads. Strings are the names the scene's string table holds.
#[derive(Debug, Clone, PartialEq)]
pub enum NisEvent {
    Car {
        system: u32,
        event: CarEvent,
    },
    /// `ECellCall`: a phone call or message, by its id.
    CellCall(u32),
    /// `ENISScreenFlash`: a full-screen flash that fades over `length` seconds.
    ScreenFlash {
        length: f32,
    },
    /// `ENISOverlayMessage`: a front-end overlay package and the message it shows.
    Overlay {
        package: String,
        message: String,
    },
    /// `ENISHideCharacter`: a named character model shown or hidden.
    HideCharacter {
        name: String,
        show: bool,
    },
    /// `ENISFreeze`: freezes or unfreezes the scene's animation.
    Freeze(bool),
    /// `ENISCopLights`: the cop lights on or off.
    CopLights(bool),
    /// `ENISPlayEffect`: a named effect at a point of the scene, with a magnitude vector.
    PlayEffect {
        name: String,
        position: [f32; 3],
        magnitude: [f32; 3],
    },
    /// `ENISMotionBlur`: the motion blur of the frame on or off.
    MotionBlur(bool),
    /// `ENISRain`: the rain intensity the scene forces.
    Rain(f32),
    /// `ENISStopEffects`: every particle effect stops.
    StopEffects,
    /// `ENISAeroDynamics`: the downforce factor of the scene's cars while they move on their own.
    AeroDynamics(f32),
    /// `ECameraPhotoFinish`: the photo-finish flash of a two-car race start.
    PhotoFinish,
    /// `EDDaySpeech`: the D-Day speech line.
    DDaySpeech,
    /// `ENISRoadNoise`: the road noise of the cars off (`true`) or on.
    RoadNoise(bool),
    /// An event the original decodes but never acts on (`ENISTimeOfDay`, `ENISFakeFar`, `ENISNukeSmack`, `ENISDetail`):
    /// its name. Spec: `nis-events.md` section 2.2.
    Inert(&'static str),
    /// An event with no handler (its id). The runner logs it once per scene.
    Other(u32),
}

/// The event type hashes the player reads (vault hashes of the class names, `docs/specs/nis-events.md` section 2).
mod ids {
    pub const CELL_CALL: u32 = 0x885F_756E;
    pub const SCREEN_FLASH: u32 = 0x9F79_A4ED;
    pub const OVERLAY_MESSAGE: u32 = 0x37A3_EE72;
    pub const CAR_SHAKE: u32 = 0xFB3D_AD79;
    pub const BRAKE_LOCK: u32 = 0xA01A_259B;
    pub const NITRO: u32 = 0xEB34_8F62;
    pub const NEUTRAL_REV: u32 = 0x00EB_198B;
    pub const DETACH: u32 = 0x257A_10A8;
    pub const REATTACH: u32 = 0xCF01_B27E;
    pub const DAMAGE_RESET: u32 = 0x991D_D250;
    pub const MOTION_BLUR: u32 = 0xFC38_2D92;
    pub const RAIN: u32 = 0x6A46_5054;
    pub const STOP_EFFECTS: u32 = 0xC058_2354;
    pub const AERODYNAMICS: u32 = 0x8DA0_5B3D;
    pub const PHOTO_FINISH: u32 = 0x2096_C371;
    pub const DDAY_SPEECH: u32 = 0xA67E_8286;
    pub const ROAD_NOISE: u32 = 0xB9A0_2FA8;
    pub const TIME_OF_DAY: u32 = 0xF699_BF04;
    pub const FAKE_FAR: u32 = 0x49D3_4FF1;
    pub const NUKE_SMACK: u32 = 0xA87E_176D;
    pub const DETAIL: u32 = 0x74E9_6511;
    pub const HIDE_CHARACTER: u32 = 0x534B_EE88;
    pub const FREEZE: u32 = 0x7E04_5D1F;
    pub const LIGHTS: u32 = 0x5E56_5165;
    pub const COP_LIGHTS: u32 = 0x83E7_31C7;
    pub const PLAY_EFFECT: u32 = 0x58AA_E7A8;
}

/// A NIS scene's running event sequence. It keeps the scene's package and the player that runs its systems. It is
/// dropped with the scene, which stops every event that has not fired yet.
pub struct NisEvents {
    package: Arc<Package>,
    player: Player,
    /// The event ids with no handler that were already logged this scene.
    unknown: BTreeSet<u32>,
    /// The ids of the events with no handler that fired since [`NisEvents::take_unhandled`] last ran, in order. Dropped
    /// with the scene; the scene report reads them.
    unhandled: Vec<u32>,
    /// Set when the player refused to run (a bad time): the scene plays on with no more events.
    failed: bool,
}

impl NisEvents {
    /// The sequence of the scene `scene`, started on the `START` stimulus. The engine is the one named after the scene,
    /// or the first engine when none is. `None` when the package has no engine or cannot start.
    pub fn start(package: Arc<Package>, scene: &str) -> Option<Self> {
        if package.engines.is_empty() {
            return None;
        }
        let engine = package.engines.iter().position(|engine| engine.name.eq_ignore_ascii_case(scene)).unwrap_or(0);
        let mut player = match Player::new(&package, engine, TAGS) {
            Ok(player) => player,
            Err(error) => {
                log::warn!("NIS {scene}: its event engine cannot run: {error}");
                return None;
            }
        };
        if let Err(error) = player.stimulate(&package, None, START, QueueMode::Allow, None) {
            log::warn!("NIS {scene}: the START stimulus failed: {error}");
            return None;
        }
        Some(Self { package, player, unknown: BTreeSet::new(), unhandled: Vec::new(), failed: false })
    }

    /// Runs `dt` seconds of scene time and returns the events that fired in it, in order. Each event fires once. The
    /// time-zero events fire on the first call, even when `dt` is zero.
    pub fn advance(&mut self, dt: f32) -> Vec<NisEvent> {
        if self.failed {
            return Vec::new();
        }
        let package = Arc::clone(&self.package);
        if let Err(error) = self.player.advance(&package, f64::from(dt)) {
            log::warn!("NIS event sequence stopped: {error}");
            self.failed = true;
            return Vec::new();
        }
        let mut fired = Vec::new();
        while let Some(emission) = self.player.pop() {
            let Some(list) = package.event_lists.get(&emission.list) else { continue };
            for event in list {
                self.collect(&package, event, emission.system, &mut fired);
            }
        }
        fired
    }

    /// The ids of the events with no handler that fired since the last call, in order.
    pub fn take_unhandled(&mut self) -> Vec<u32> {
        std::mem::take(&mut self.unhandled)
    }

    /// Decodes one event into `fired`. An event with no handler is logged once; a malformed one is logged and skipped.
    fn collect(&mut self, package: &Package, event: &Event, system: u32, fired: &mut Vec<NisEvent>) {
        match decode(package, event) {
            Some(NisEvent::Other(id)) => {
                self.unhandled.push(id);
                if self.unknown.insert(id) {
                    log::debug!("NIS event {id:08X} has no handler; it is ignored");
                }
            }
            Some(NisEvent::Car { event, .. }) => fired.push(NisEvent::Car { system, event }),
            Some(decoded) => fired.push(decoded),
            None => log::warn!("NIS event {:08X} is malformed; it is skipped", event.id),
        }
    }
}

/// Decodes an event's static data (its id is the first word). `None` when the data is too short for the event's fields.
pub fn decode(package: &Package, event: &Event) -> Option<NisEvent> {
    let fields = Fields { data: &event.data, package };
    if let Some(car) = decode_car(&fields, event.id) {
        return Some(NisEvent::Car { system: 0, event: car? });
    }
    let decoded = match event.id {
        ids::CELL_CALL => NisEvent::CellCall(fields.word(4)?),
        ids::SCREEN_FLASH => NisEvent::ScreenFlash { length: fields.number(4)? },
        ids::OVERLAY_MESSAGE => NisEvent::Overlay { package: fields.text(4)?, message: fields.text(8)? },
        ids::HIDE_CHARACTER => NisEvent::HideCharacter { name: fields.text(4)?, show: fields.word(8)? != 0 },
        ids::FREEZE => NisEvent::Freeze(fields.word(4)? != 0),
        ids::COP_LIGHTS => NisEvent::CopLights(fields.word(4)? != 0),
        ids::PLAY_EFFECT => NisEvent::PlayEffect {
            name: fields.text(4)?,
            position: [fields.number(8)?, fields.number(12)?, fields.number(16)?],
            magnitude: [fields.number(20)?, fields.number(24)?, fields.number(28)?],
        },
        ids::MOTION_BLUR => NisEvent::MotionBlur(fields.word(4)? != 0),
        ids::RAIN => NisEvent::Rain(fields.number(4)?),
        ids::STOP_EFFECTS => NisEvent::StopEffects,
        ids::AERODYNAMICS => NisEvent::AeroDynamics(fields.number(4)?),
        ids::PHOTO_FINISH => NisEvent::PhotoFinish,
        ids::DDAY_SPEECH => NisEvent::DDaySpeech,
        ids::ROAD_NOISE => NisEvent::RoadNoise(fields.word(4)? != 0),
        ids::TIME_OF_DAY => NisEvent::Inert("ENISTimeOfDay"),
        ids::FAKE_FAR => NisEvent::Inert("ENISFakeFar"),
        ids::NUKE_SMACK => NisEvent::Inert("ENISNukeSmack"),
        ids::DETAIL => NisEvent::Inert("ENISDetail"),
        id => NisEvent::Other(id),
    };
    Some(decoded)
}

/// A car event (one that acts on the car of the channel whose system fires it): `None` when `id` is not one, `Some(None)`
/// when its data is too short.
fn decode_car(fields: &Fields<'_>, id: u32) -> Option<Option<CarEvent>> {
    let event = match id {
        0xA18E_1896 => fields.number(4).zip(fields.number(8)).map(|(dip, length)| CarEvent::Pitch { dip, length }),
        0x3081_837D => fields.number(4).zip(fields.number(8)).map(|(dip, length)| CarEvent::Roll { dip, length }),
        0x6A52_65B2 => fields.number(4).map(CarEvent::Burnout),
        ids::CAR_SHAKE => (|| {
            Some(CarEvent::Shake {
                dip: fields.number(4)?,
                rate: fields.number(8)?,
                ramp: fields.number(12)?,
                time: fields.number(16)?,
            })
        })(),
        ids::BRAKE_LOCK => fields
            .word(4)
            .zip(fields.word(8))
            .map(|(front, rear)| CarEvent::BrakeLock { front: front != 0, rear: rear != 0 }),
        ids::LIGHTS => {
            fields.word(4).zip(fields.word(8)).map(|(cop, head)| CarEvent::Lights { cop: cop != 0, head: head != 0 })
        }
        ids::NITRO => fields.word(4).map(|on| CarEvent::Nitro(on != 0)),
        ids::NEUTRAL_REV => (|| {
            Some(CarEvent::NeutralRev {
                on: fields.word(4)? != 0,
                throttle: fields.number(8)?,
                speed: fields.number(12)?,
            })
        })(),
        ids::DETACH => Some(CarEvent::Detach),
        ids::REATTACH => fields.word(4).map(|repair| CarEvent::Reattach { repair: repair != 0 }),
        ids::DAMAGE_RESET => Some(CarEvent::DamageReset),
        _ => return None,
    };
    Some(event)
}

/// The fields of one event's static data, read little-endian.
struct Fields<'a> {
    data: &'a [u8],
    package: &'a Package,
}

impl Fields<'_> {
    fn word(&self, at: usize) -> Option<u32> {
        let bytes: [u8; 4] = self.data.get(at..at + 4)?.try_into().ok()?;
        Some(u32::from_le_bytes(bytes))
    }

    /// A float field. A value that is not finite is malformed.
    fn number(&self, at: usize) -> Option<f32> {
        let value = f32::from_bits(self.word(at)?);
        value.is_finite().then_some(value)
    }

    /// A string field: the tag of a string record in the package. A tag with no string reads as empty.
    fn text(&self, at: usize) -> Option<String> {
        let tag = self.word(at)?;
        Some(self.package.strings.get(&tag).cloned().unwrap_or_default())
    }
}

#[cfg(test)]
#[path = "events_tests.rs"]
mod tests;
