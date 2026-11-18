//! The drive to the first safe house, as a state machine (`docs/specs/story-flow.md` section 2.5;
//! `docs/specs/career-scripts-events.md` section 7.1; F-17 and F-18 in `docs/specs/story-flow-2.md` section 6).
//!
//! Race bin 16 runs this as its entry activity after the car lot (`race_bin_16/drive_to_safe_house`, a copy of the
//! `drive_to_target` activity). The states are driving, at the target and done, and the caller plays the [`DriveEvent`]s
//! it gets back:
//!
//! - [`DriveToSafeHouse::start`] is the initial state: cops off, then activate_gps when the activity's `AutoActivateGPS`
//!   is set (the GPS to the target with the default allowance), and message 15 unless the career intro is skipped. It then
//!   enters driving; a skipped career intro goes on to done at once.
//! - [`DriveToSafeHouse::update`] runs the driving timers, each once: `cellusage` at 0.25 s shows the GPS to the target
//!   with an allowance of 2000, and `hurry` at 120 s sends message 121.
//! - [`DriveToSafeHouse::arrive`] is the `target` trigger (`gpstarget`). The timers stop and `DoSafeHouseIntro` runs with
//!   the respawn marker. The graph has no transition from there to done.
//! - [`DriveToSafeHouse::finish`] is the done state: cops back on, GPS off. The caller calls it when the bin's activity
//!   ends. The spec's `outro_movie` state is never entered (F-17), so it is not modelled.

use crate::phone::FIRST_DRIVE_SMS;
use crate::settings::Flags;

/// The safe house respawn marker: the marker `DoSafeHouseIntro` is called with (the activity's `RespawnMarker`, F-17).
pub const SAFE_HOUSE_RESPAWN_MARKER: &str = "menu_gate_safehouse/zone_safehouse_1/respawn";
/// The drive's target: the activity's `target`, its child box trigger `gpstarget`. The GPS leads to it and entering it is
/// the arrival.
pub const GPS_TARGET: &str = "race_bin_16/drive_to_safe_house/gpstarget";
/// The activity's `AutoActivateGPS` in the install: `drive_to_safe_house` sets it false (the `drive_to_target` template
/// sets it true), so the drive's GPS comes from the `cellusage` timer, not from activate_gps.
pub const SAFE_HOUSE_AUTO_GPS: bool = false;
/// The message sent after [`HURRY_SECS`] of driving (`hurry`).
pub const HURRY_SMS: u32 = 121;
/// The time after which [`HURRY_SMS`] is sent, counted from the start of driving.
pub const HURRY_SECS: f32 = 120.0;
/// When the `cellusage` timer shows the GPS, counted from the start of driving. The timer is killed when it fires.
pub const GPS_SHOW_SECS: f32 = 0.25;
/// The third argument of the `cellusage` timer's `NavigatePlayerTo`; activate_gps passes 0 there. Read as the GPS's
/// largest deviation from the route in metres, whose default of 200 m an argument below 0.1 asks for
/// (`docs/specs/ai-pathfinder.md` section 7) **[unconfirmed]**: the native call is not read.
pub const GPS_SHOW_DEVIATION_M: f32 = 2000.0;

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
    /// Show the GPS and navigate to the target with the minimap item (`NavigatePlayerTo`): from activate_gps with an
    /// allowance of 0 (the default), from the `cellusage` timer with [`GPS_SHOW_DEVIATION_M`].
    GpsNavigate {
        /// The target trigger.
        target: &'static str,
        /// The largest deviation from the route, metres; below 0.1 the GPS's default.
        max_deviation_m: f32,
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
    gps_shown: bool,
    hurry_sent: bool,
}

/// True when the career intro is skipped: the debug flag is set, or the opening was already seen (`0x20`, CF 6).
pub fn skip_career_intro(debug_flag: bool, flags: Flags) -> bool {
    debug_flag || flags.contains(Flags::OPENING_SEEN)
}

impl DriveToSafeHouse {
    /// Runs the initial state and returns the drive in the driving state with the events of the entry actions.
    ///
    /// Cops go off. With `auto_gps` (the activity's `AutoActivateGPS`) the GPS navigates to the target. Message 15 is sent
    /// unless the career intro is skipped (`skip_career_intro`); a skipped intro also ends the drive at once, as driving's
    /// entry goes to done then.
    pub fn start(auto_gps: bool, skip_career_intro: bool) -> (DriveToSafeHouse, Vec<DriveEvent>) {
        let mut events = vec![DriveEvent::CopsOff];
        if auto_gps {
            events.push(DriveEvent::GpsNavigate { target: GPS_TARGET, max_deviation_m: 0.0 });
        }
        let mut drive =
            DriveToSafeHouse { state: DriveState::Driving, driving_secs: 0.0, gps_shown: false, hurry_sent: false };
        if skip_career_intro {
            drive.state = DriveState::Done;
            events.extend([DriveEvent::CopsOn, DriveEvent::GpsOff]);
            return (drive, events);
        }
        events.push(DriveEvent::AddSms(FIRST_DRIVE_SMS));
        (drive, events)
    }

    /// The current state.
    pub fn state(&self) -> DriveState {
        self.state
    }

    /// Advances the driving timers by `dt` seconds. Nothing happens outside the driving state. Each timer fires once.
    pub fn update(&mut self, dt: f32) -> Vec<DriveEvent> {
        let mut events = Vec::new();
        if self.state != DriveState::Driving {
            return events;
        }
        self.driving_secs += dt;
        if !self.gps_shown && self.driving_secs >= GPS_SHOW_SECS {
            self.gps_shown = true;
            events.push(DriveEvent::GpsNavigate { target: GPS_TARGET, max_deviation_m: GPS_SHOW_DEVIATION_M });
        }
        if !self.hurry_sent && self.driving_secs >= HURRY_SECS {
            self.hurry_sent = true;
            events.push(DriveEvent::AddSms(HURRY_SMS));
        }
        events
    }

    /// The `target` trigger is entered. From driving, the timers stop and the safe house intro is called. Any other
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

    fn navigate(max_deviation_m: f32) -> DriveEvent {
        DriveEvent::GpsNavigate { target: GPS_TARGET, max_deviation_m }
    }

    #[test]
    fn the_target_is_the_gps_target_box_and_the_intro_marker_the_respawn() {
        assert_eq!(GPS_TARGET, "race_bin_16/drive_to_safe_house/gpstarget");
        assert_eq!(SAFE_HOUSE_RESPAWN_MARKER, "menu_gate_safehouse/zone_safehouse_1/respawn");
    }

    #[test]
    fn a_new_career_turns_cops_off_and_sends_message_15_then_drives() {
        let (drive, events) = DriveToSafeHouse::start(SAFE_HOUSE_AUTO_GPS, false);
        assert_eq!(events, vec![DriveEvent::CopsOff, DriveEvent::AddSms(15)]);
        assert_eq!(drive.state(), DriveState::Driving);
    }

    #[test]
    fn auto_gps_navigates_to_the_target_before_message_15() {
        let (_, events) = DriveToSafeHouse::start(true, false);
        assert_eq!(events, vec![DriveEvent::CopsOff, navigate(0.0), DriveEvent::AddSms(15)]);
    }

    #[test]
    fn a_skipped_career_intro_sends_no_message_15_and_ends_the_drive() {
        let (drive, events) = DriveToSafeHouse::start(false, true);
        assert_eq!(events, vec![DriveEvent::CopsOff, DriveEvent::CopsOn, DriveEvent::GpsOff]);
        assert_eq!(drive.state(), DriveState::Done);
        let (_, events) = DriveToSafeHouse::start(true, true);
        assert_eq!(events, vec![DriveEvent::CopsOff, navigate(0.0), DriveEvent::CopsOn, DriveEvent::GpsOff]);
    }

    #[test]
    fn the_intro_is_skipped_by_the_debug_flag_or_the_opening_flag() {
        assert!(skip_career_intro(true, Flags::EMPTY));
        assert!(skip_career_intro(false, Flags::OPENING_SEEN));
        assert!(!skip_career_intro(false, Flags::EMPTY));
        assert!(!skip_career_intro(false, Flags::CAREER_STARTED));
    }

    #[test]
    fn the_gps_is_shown_once_a_quarter_second_into_the_drive_with_its_2000_allowance() {
        let (mut drive, _) = DriveToSafeHouse::start(SAFE_HOUSE_AUTO_GPS, false);
        assert!(drive.update(0.125).is_empty(), "nothing before 0.25 s");
        assert_eq!(drive.update(0.125), vec![navigate(GPS_SHOW_DEVIATION_M)]);
        for _ in 0..40 {
            assert!(drive.update(0.25).is_empty(), "the cellusage timer is killed when it fires");
        }
    }

    #[test]
    fn a_long_first_frame_shows_the_gps_once() {
        let (mut drive, _) = DriveToSafeHouse::start(SAFE_HOUSE_AUTO_GPS, false);
        assert_eq!(drive.update(1.0), vec![navigate(GPS_SHOW_DEVIATION_M)]);
        assert!(drive.update(0.0).is_empty());
    }

    #[test]
    fn message_121_is_sent_once_at_120_s() {
        let (mut drive, _) = DriveToSafeHouse::start(SAFE_HOUSE_AUTO_GPS, false);
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
        let (mut drive, _) = DriveToSafeHouse::start(SAFE_HOUSE_AUTO_GPS, false);
        assert_eq!(drive.arrive(), vec![DriveEvent::SafeHouseIntro { marker: SAFE_HOUSE_RESPAWN_MARKER }]);
        assert_eq!(drive.state(), DriveState::AtTarget);
        let mut after = Vec::new();
        for _ in 0..200 {
            after.extend(drive.update(1.0));
        }
        assert!(after.is_empty(), "no GPS and no hurry after the target");
    }

    #[test]
    fn finish_turns_cops_back_on_and_the_gps_off_once() {
        let (mut drive, _) = DriveToSafeHouse::start(SAFE_HOUSE_AUTO_GPS, false);
        assert!(drive.finish().is_empty(), "done is only entered from the target");
        drive.arrive();
        assert_eq!(drive.finish(), vec![DriveEvent::CopsOn, DriveEvent::GpsOff]);
        assert_eq!(drive.state(), DriveState::Done);
        assert!(drive.finish().is_empty());
        assert!(drive.arrive().is_empty());
    }
}
