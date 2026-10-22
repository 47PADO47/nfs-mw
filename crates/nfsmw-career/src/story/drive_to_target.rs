//! The drive to the first safe house, as a state machine (`docs/specs/story-flow.md` section 2.5;
//! `docs/specs/career-scripts-events.md` section 7.1; F-17 and F-18 in `docs/specs/story-flow-2.md` section 6).
//!
//! Race bin 16 runs this as its entry activity after the car lot. The states are driving, at the target and done, and
//! the caller plays the [`DriveEvent`]s it gets back:
//!
//! - [`DriveToSafeHouse::start`] is the initial state: cops off, the GPS to the target when GPS is set to auto (the
//!   activate_gps state), then message 15 unless the career intro is skipped. It then enters driving.
//! - [`DriveToSafeHouse::update`] runs the driving timers: the GPS refresh every 0.25 s (`cellusage`), and message 121
//!   once after 120 s (`hurry`).
//! - [`DriveToSafeHouse::arrive`] is the `gpstarget` trigger. The timers stop and `DoSafeHouseIntro` runs with the
//!   respawn marker. The spec has no transition from there to done.
//! - [`DriveToSafeHouse::finish`] is the done state: cops back on, GPS off. The caller calls it when the bin's activity
//!   ends. The spec's `outro_movie` state is never entered (F-17), so it is not modelled.

use crate::phone::FIRST_DRIVE_SMS;
use crate::settings::Flags;

/// The safe house respawn marker: the target of the drive and the marker `DoSafeHouseIntro` is called with (F-17).
pub const SAFE_HOUSE_RESPAWN_MARKER: &str = "menu_gate_safehouse/zone_safehouse_1/respawn";
/// The message sent after [`HURRY_SECS`] of driving (`hurry`).
pub const HURRY_SMS: u32 = 121;
/// The time after which [`HURRY_SMS`] is sent, counted from the start of driving.
pub const HURRY_SECS: f32 = 120.0;
/// The period of the GPS refresh (`cellusage`).
pub const GPS_REFRESH_SECS: f32 = 0.25;
/// The offset of the second GPS hint from the target, in metres (section 2.5).
pub const GPS_HINT_OFFSET_M: f32 = 2000.0;

/// The states of the drive (section 2.5). The initial state and activate_gps are only entered in
/// [`DriveToSafeHouse::start`], so they are not kept.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriveState {
    /// Driving to the target: the GPS and the timers run.
    Driving,
    /// The target is reached and the safe house intro has been called.
    AtTarget,
    /// The activity is done: cops on, GPS off.
    Done,
}

/// What the drive asks the game to do.
#[derive(Debug, Clone, PartialEq)]
pub enum DriveEvent {
    /// Cops are switched off.
    CopsOff,
    /// Cops are switched on.
    CopsOn,
    /// `EAddSMS` with the message id.
    AddSms(u32),
    /// Show the GPS to the target and navigate there with the minimap item (activate_gps).
    GpsNavigate {
        /// The target marker.
        target: &'static str,
    },
    /// Show the GPS to the target, or a hint at an offset from it (`cellusage`).
    GpsShow {
        /// The target marker.
        target: &'static str,
        /// The offset of the hint from the target, in metres; 0 for the target itself.
        offset_m: f32,
    },
    /// The GPS is switched off.
    GpsOff,
    /// `DoSafeHouseIntro(marker)`: the safe house intro takes over from here.
    SafeHouseIntro {
        /// The respawn marker.
        marker: &'static str,
    },
}

/// The drive to the safe house. Build it with [`DriveToSafeHouse::start`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DriveToSafeHouse {
    state: DriveState,
    /// Seconds of driving so far.
    driving_secs: f32,
    /// Seconds since the last GPS refresh, kept below the refresh period.
    since_refresh: f32,
    hurry_sent: bool,
}

/// True when the career intro is skipped: the debug flag is set, or the opening was already seen (`0x20`, CF 6).
pub fn skip_career_intro(debug_flag: bool, flags: Flags) -> bool {
    debug_flag || flags.contains(Flags::OPENING_SEEN)
}

impl DriveToSafeHouse {
    /// Runs the initial state and returns the drive in the driving state with the events of the entry actions.
    ///
    /// Cops go off. With `auto_gps` the GPS navigates to the target. Message 15 is sent unless the career intro is
    /// skipped (`skip_career_intro`).
    pub fn start(auto_gps: bool, skip_career_intro: bool) -> (DriveToSafeHouse, Vec<DriveEvent>) {
        let mut events = vec![DriveEvent::CopsOff];
        if auto_gps {
            events.push(DriveEvent::GpsNavigate { target: SAFE_HOUSE_RESPAWN_MARKER });
        }
        if !skip_career_intro {
            events.push(DriveEvent::AddSms(FIRST_DRIVE_SMS));
        }
        let drive =
            DriveToSafeHouse { state: DriveState::Driving, driving_secs: 0.0, since_refresh: 0.0, hurry_sent: false };
        (drive, events)
    }

    /// The current state.
    pub fn state(&self) -> DriveState {
        self.state
    }

    /// Advances the driving timers by `dt` seconds. Nothing happens outside the driving state.
    ///
    /// The GPS refresh fires once per update in which at least one period has passed, since a refresh does not
    /// stack. The hurry message fires once.
    pub fn update(&mut self, dt: f32) -> Vec<DriveEvent> {
        let mut events = Vec::new();
        if self.state != DriveState::Driving {
            return events;
        }
        self.driving_secs += dt;
        self.since_refresh += dt;
        let mut due = false;
        while self.since_refresh >= GPS_REFRESH_SECS {
            self.since_refresh -= GPS_REFRESH_SECS;
            due = true;
        }
        if due {
            events.push(DriveEvent::GpsShow { target: SAFE_HOUSE_RESPAWN_MARKER, offset_m: 0.0 });
            events.push(DriveEvent::GpsShow { target: SAFE_HOUSE_RESPAWN_MARKER, offset_m: GPS_HINT_OFFSET_M });
        }
        if !self.hurry_sent && self.driving_secs >= HURRY_SECS {
            self.hurry_sent = true;
            events.push(DriveEvent::AddSms(HURRY_SMS));
        }
        events
    }

    /// The `gpstarget` trigger is entered. From driving, the timers stop and the safe house intro is called. Any other
    /// state ignores it.
    pub fn arrive(&mut self) -> Vec<DriveEvent> {
        if self.state != DriveState::Driving {
            return Vec::new();
        }
        self.state = DriveState::AtTarget;
        vec![DriveEvent::SafeHouseIntro { marker: SAFE_HOUSE_RESPAWN_MARKER }]
    }

    /// The done state: cops back on and the GPS off. Only leaves [`DriveState::AtTarget`].
    pub fn finish(&mut self) -> Vec<DriveEvent> {
        if self.state != DriveState::AtTarget {
            return Vec::new();
        }
        self.state = DriveState::Done;
        vec![DriveEvent::CopsOn, DriveEvent::GpsOff]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn show(offset_m: f32) -> DriveEvent {
        DriveEvent::GpsShow { target: SAFE_HOUSE_RESPAWN_MARKER, offset_m }
    }

    #[test]
    fn the_target_is_the_respawn_marker_of_the_safe_house() {
        assert_eq!(SAFE_HOUSE_RESPAWN_MARKER, "menu_gate_safehouse/zone_safehouse_1/respawn");
    }

    #[test]
    fn a_new_career_turns_cops_off_and_sends_message_15_then_drives() {
        let (drive, events) = DriveToSafeHouse::start(false, false);
        assert_eq!(events, vec![DriveEvent::CopsOff, DriveEvent::AddSms(15)]);
        assert_eq!(drive.state(), DriveState::Driving);
    }

    #[test]
    fn auto_gps_navigates_to_the_target_before_message_15() {
        let (_, events) = DriveToSafeHouse::start(true, false);
        assert_eq!(
            events,
            vec![
                DriveEvent::CopsOff,
                DriveEvent::GpsNavigate { target: SAFE_HOUSE_RESPAWN_MARKER },
                DriveEvent::AddSms(15),
            ]
        );
    }

    #[test]
    fn a_skipped_career_intro_sends_no_message_15() {
        let (_, events) = DriveToSafeHouse::start(false, true);
        assert_eq!(events, vec![DriveEvent::CopsOff]);
        let (_, events) = DriveToSafeHouse::start(true, true);
        assert_eq!(events, vec![DriveEvent::CopsOff, DriveEvent::GpsNavigate { target: SAFE_HOUSE_RESPAWN_MARKER }]);
    }

    #[test]
    fn the_intro_is_skipped_by_the_debug_flag_or_the_opening_flag() {
        assert!(skip_career_intro(true, Flags::EMPTY));
        assert!(skip_career_intro(false, Flags::OPENING_SEEN));
        assert!(!skip_career_intro(false, Flags::EMPTY));
        assert!(!skip_career_intro(false, Flags::CAREER_STARTED));
    }

    #[test]
    fn the_gps_refreshes_every_quarter_second_with_the_hint_at_2000_m() {
        let (mut drive, _) = DriveToSafeHouse::start(false, true);
        assert!(drive.update(0.125).is_empty(), "no refresh before 0.25 s");
        assert_eq!(drive.update(0.125), vec![show(0.0), show(GPS_HINT_OFFSET_M)]);
        assert!(drive.update(0.125).is_empty());
        assert_eq!(drive.update(0.125), vec![show(0.0), show(GPS_HINT_OFFSET_M)]);
    }

    #[test]
    fn a_long_frame_gives_one_refresh_not_one_per_period() {
        let (mut drive, _) = DriveToSafeHouse::start(false, true);
        assert_eq!(drive.update(1.0), vec![show(0.0), show(GPS_HINT_OFFSET_M)]);
        assert!(drive.update(0.0).is_empty());
    }

    #[test]
    fn message_121_is_sent_once_at_120_s() {
        let (mut drive, _) = DriveToSafeHouse::start(false, true);
        let mut hurry = Vec::new();
        for frame in 1..=130 {
            let events = drive.update(1.0);
            for event in events {
                if event == DriveEvent::AddSms(HURRY_SMS) {
                    hurry.push(frame);
                }
            }
        }
        assert_eq!(hurry, vec![120]);
    }

    #[test]
    fn entering_the_target_calls_the_safe_house_intro_and_stops_the_timers() {
        let (mut drive, _) = DriveToSafeHouse::start(false, true);
        drive.update(1.0);
        assert_eq!(drive.arrive(), vec![DriveEvent::SafeHouseIntro { marker: SAFE_HOUSE_RESPAWN_MARKER }]);
        assert_eq!(drive.state(), DriveState::AtTarget);
        let mut after = Vec::new();
        for _ in 0..200 {
            after.extend(drive.update(1.0));
        }
        assert!(after.is_empty(), "no refresh and no hurry after the target");
    }

    #[test]
    fn finish_turns_cops_back_on_and_the_gps_off_once() {
        let (mut drive, _) = DriveToSafeHouse::start(false, true);
        assert!(drive.finish().is_empty(), "done is only entered from the target");
        drive.arrive();
        assert_eq!(drive.finish(), vec![DriveEvent::CopsOn, DriveEvent::GpsOff]);
        assert_eq!(drive.state(), DriveState::Done);
        assert!(drive.finish().is_empty());
        assert!(drive.arrive().is_empty());
    }

    #[test]
    fn a_full_drive_plays_its_events_in_order() {
        let (mut drive, start) = DriveToSafeHouse::start(false, false);
        let mut events = start;
        events.extend(drive.update(0.25));
        events.extend(drive.arrive());
        events.extend(drive.finish());
        assert_eq!(
            events,
            vec![
                DriveEvent::CopsOff,
                DriveEvent::AddSms(15),
                show(0.0),
                show(GPS_HINT_OFFSET_M),
                DriveEvent::SafeHouseIntro { marker: SAFE_HOUSE_RESPAWN_MARKER },
                DriveEvent::CopsOn,
                DriveEvent::GpsOff,
            ]
        );
    }
}
