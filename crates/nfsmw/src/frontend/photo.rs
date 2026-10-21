//! The win photo (`InGamePhotoMaster.fng`): the screen after a race win (the Lua graph's `ShowWinningPostRaceScreen`), and
//! the photo of a speed trap in free roam. One group of the package is shown by the kind of result: `SPRINT_GROUP`,
//! `RIVAL_GROUP` (used here for drag races), `TOLL_BOOTH_GROUP` or `SPEEDTRAP_GROUP`. The frozen world behind it is the
//! photo. After a race win the buttons are Race Stats (the standings page of `post_race/`), Restart (which asks OK or
//! Cancel first) and Continue; after a trap only Continue. `docs/provenance/race-mode.md` section 10 has what is known of the package.

use blackbox_feng::ids::{PAD_ACCEPT, PAD_BUTTON0};

use super::confirm::Confirm;
use super::logic::{Command, Cx, ScreenLogic};
use crate::settings::SpeedUnits;
use crate::viewer::{PhotoKind, PhotoReport, RaceAnswer, RaceReport};

/// The groups, by name hash, and the script that fades a group in (1800 ms; the groups rest hidden).
const SPRINT_GROUP: u32 = 0xE057_94EB;
const RIVAL_GROUP: u32 = 0x895E_5289;
const TOLL_BOOTH_GROUP: u32 = 0xC591_84C1;
const SPEEDTRAP_GROUP: u32 = 0xFBE6_5EB3;
const FADE_IN: u32 = 0xBCBF_CC87;
/// The package's event handler string (`EVENT HANDLER`) and its entrance script (500 ms: `0xBDFD2DF9` to the parts,
/// `INIT_COMPLETE`, and the message that switches the buttons on).
const EVENT_HANDLER: u32 = 0x47FF_4E7C;
const ENTRANCE: u32 = 0x0013_C37B;
/// The speed trap's camera frame (`ROCKPORT POLICE VID CAM 01`), shown with its group.
const CAMERA_FRAME: u32 = 0x857F_B472;
/// The picture the package ships for the photo (`photofinish_toobooth.tga`): the frozen world is the photo here.
const PLACEHOLDER_PICTURE: u32 = 0x286A_9CD4;
/// The strings: the cash line and the time line of the sprint, rival and tollbooth groups, and the speed trap's.
const SPRINT_CASH: u32 = 0x3D17_73DD;
const SPRINT_TIME: u32 = 0xAB6A_AFDD;
const RIVAL_CASH: u32 = 0x9F4D_F5BB;
const RIVAL_TIME: u32 = 0x37BE_A03B;
const TOLL_SPEED: u32 = 0x8BB3_9726;
const TOLL_TIME: u32 = 0x424B_B244;
const TOLL_CASH: u32 = 0x4242_3E94;
const TOLL_BONUS: u32 = 0x8A7F_929C;
const TRAP_SPEED: u32 = 0xFCBE_3F57;
const TRAP_BOUNTY: u32 = 0x3233_F0A4;
/// The messages of the footer buttons: Race Stats and Restart (`PAD_BUTTON4` and `PAD_BUTTON5`).
const STATS: u32 = PAD_BUTTON0 + 4;
const RESTART: u32 = PAD_BUTTON0 + 5;

/// Shows a [`PhotoReport`], with the race's report behind its Race Stats button.
pub struct PhotoLogic {
    photo: PhotoReport,
    report: Option<RaceReport>,
    /// The group has been faded in: the package's own first script rests it hidden, so the fade starts after the first
    /// frame.
    faded: bool,
}

impl PhotoLogic {
    /// The photo of a won race, with the lap table behind Race Stats.
    pub fn for_race(photo: PhotoReport, report: RaceReport) -> Self {
        Self { photo, report: Some(report), faded: false }
    }

    /// The photo of a speed trap: Continue only.
    pub fn for_trap(photo: PhotoReport) -> Self {
        Self { photo, report: None, faded: false }
    }

    fn group(&self) -> u32 {
        match self.photo.kind {
            PhotoKind::Sprint => SPRINT_GROUP,
            PhotoKind::Drag => RIVAL_GROUP,
            PhotoKind::Tollbooth => TOLL_BOOTH_GROUP,
            PhotoKind::SpeedTrap => SPEEDTRAP_GROUP,
        }
    }

    fn fill(&self, cx: &mut Cx) {
        let p = &self.photo;
        let units = cx.settings.speed_units;
        let cash = format!("CASH {}", p.cash);
        match p.kind {
            PhotoKind::Sprint => {
                cx.text_object(SPRINT_CASH, &cash);
                cx.text_object(SPRINT_TIME, &format!("{} @ {}", clock(p.time), speed(p.speed, units)));
            }
            PhotoKind::Drag => {
                cx.text_object(RIVAL_CASH, &cash);
                cx.text_object(RIVAL_TIME, &format!("{:.1} sec. @ {}", p.time, speed(p.speed, units)));
            }
            PhotoKind::Tollbooth => {
                cx.text_object(TOLL_SPEED, &speed(p.speed, units));
                cx.text_object(TOLL_TIME, &clock(p.time_left));
                cx.text_object(TOLL_CASH, &cash);
                cx.text_object(TOLL_BONUS, &format!("+{}", clock_short(p.bonus_secs)));
            }
            PhotoKind::SpeedTrap => {
                cx.text_object(TRAP_SPEED, &speed(p.speed, units));
                cx.text_object(TRAP_BOUNTY, &format!("BOUNTY: {}", p.bounty));
            }
        }
    }

    fn fade_in(&self, cx: &mut Cx) {
        if let Some(group) = cx.object(self.group()) {
            cx.rt.run_script(group, FADE_IN);
        }
        if self.photo.kind == PhotoKind::SpeedTrap
            && let Some(frame) = cx.object(CAMERA_FRAME)
        {
            cx.rt.run_script(frame, FADE_IN);
        }
    }

    /// Keeps every other group of the package hidden, and the shipped picture off the frozen world.
    fn hold(&self, cx: &mut Cx) {
        let shown = self.group();
        for group in [SPRINT_GROUP, RIVAL_GROUP, TOLL_BOOTH_GROUP, SPEEDTRAP_GROUP] {
            cx.hide(group, group != shown);
        }
        cx.hide(CAMERA_FRAME, self.photo.kind != PhotoKind::SpeedTrap);
        cx.hide(PLACEHOLDER_PICTURE, true);
        // The group's own fade does not leave it visible (its colour stays at 0 after the script), so it is held at full
        // alpha, as the HUD holds its groups; its parts keep their scripts.
        for hash in [shown, CAMERA_FRAME] {
            let on = hash == shown || self.photo.kind == PhotoKind::SpeedTrap;
            if let (true, Some(group)) = (on, cx.object(hash)) {
                cx.rt.set_alpha(group, 255);
            }
        }
    }
}

/// `m:ss:cc`, the package's time text (`1:20:50`).
fn clock(secs: f32) -> String {
    let hundredths = (secs.max(0.0) * 100.0).round() as u64;
    format!("{}:{:02}:{:02}", hundredths / 6000, (hundredths / 100) % 60, hundredths % 100)
}

/// `m:ss`, for the tollbooth bonus.
fn clock_short(secs: f32) -> String {
    let whole = secs.max(0.0).round() as u64;
    format!("{}:{:02}", whole / 60, whole % 60)
}

/// A speed in the player's unit.
fn speed(mps: f32, units: SpeedUnits) -> String {
    match units {
        SpeedUnits::Metric => format!("{:.0} KM/H", mps * 3.6),
        SpeedUnits::Imperial => format!("{:.0} MPH", mps * 2.236_936),
    }
}

impl ScreenLogic for PhotoLogic {
    fn start(&mut self, cx: &mut Cx) {
        self.fill(cx);
        self.hold(cx);
    }

    fn tick(&mut self, cx: &mut Cx, _dt: f32) {
        self.hold(cx);
        if !self.faded {
            self.faded = true;
            // The package's entrance script, on its event handler: it sends the message the footer and the backing fade in
            // on, the end of the entrance and the one that switches the buttons on.
            if let Some(handler) = cx.object(EVENT_HANDLER) {
                cx.rt.run_script(handler, ENTRANCE);
            }
            self.fade_in(cx);
        }
    }

    fn message(&mut self, cx: &mut Cx, message: u32) {
        match (message, &self.report) {
            (PAD_ACCEPT, Some(_)) => cx.send(Command::RaceAnswer(RaceAnswer::Exit)),
            (PAD_ACCEPT, None) => cx.send(Command::Resume),
            (STATS, Some(report)) => cx.send(Command::ShowResults(report.clone())),
            (RESTART, Some(_)) => cx.send(Command::Confirm(Confirm::restart_results())),
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_package_times_read_minutes_seconds_hundredths() {
        assert_eq!(clock(80.5), "1:20:50");
        assert_eq!(clock(0.0), "0:00:00");
        assert_eq!(clock_short(35.0), "0:35");
    }

    #[test]
    fn a_speed_follows_the_unit() {
        assert_eq!(speed(50.0, SpeedUnits::Metric), "180 KM/H");
        assert_eq!(speed(50.0, SpeedUnits::Imperial), "112 MPH");
    }

    #[test]
    fn each_kind_takes_its_own_group() {
        let photo = |kind| PhotoLogic::for_trap(PhotoReport { kind, ..PhotoReport::sample() });
        assert_eq!(photo(PhotoKind::Sprint).group(), SPRINT_GROUP);
        assert_eq!(photo(PhotoKind::Drag).group(), RIVAL_GROUP);
        assert_eq!(photo(PhotoKind::Tollbooth).group(), TOLL_BOOTH_GROUP);
        assert_eq!(photo(PhotoKind::SpeedTrap).group(), SPEEDTRAP_GROUP);
    }
}
