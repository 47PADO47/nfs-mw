//! The win photo (`InGamePhotoMaster.fng`): the screen after a race win (the Lua graph's `ShowWinningPostRaceScreen`), and
//! the photo of a speed trap in free roam. One group of the package is shown by the kind of result: `SPRINT_GROUP`,
//! `RIVAL_GROUP` (used here for drag races), `TOLL_BOOTH_GROUP` or `SPEEDTRAP_GROUP`. The frozen world behind it is the
//! photo. After a race win the buttons are Race Stats (the standings page of `post_race/`), Restart (which asks OK or
//! Cancel first) and Continue; after a trap only Continue. `docs/provenance/race-mode.md` section 10 has what is known of the package.

use blackbox_feng::fe_hash_upper;
use blackbox_feng::ids::{PAD_ACCEPT, PAD_BUTTON0};
use blackbox_feng::runtime::pad;

use super::confirm::Confirm;
use super::logic::{Command, Cx, ScreenLogic};
use crate::i18n::{self, fmt::grouped, t};
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
/// The picture of the photo (`photofinish_toobooth.tga` as shipped): a polaroid on gravel whose window is transparent, so
/// the world behind it, seen from the finish camera, is the photo. The race names the texture it is swapped for.
const PICTURE: u32 = 0x286A_9CD4;
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
/// The language labels of the cash word (`PHOTOFINISH_CASH`), the bounty line (`RAP_TOTAL_BOUNTY`, a `%$d` template) and
/// the speed units (`HUD_KMH`, `HUD_MPH`).
const CASH_WORD: u32 = 0xB7F2_B3C8;
const BOUNTY_TEMPLATE: u32 = 0x364E_4525;
const KPH_LABEL: u32 = 0x8569_A25F;
const MPH_LABEL: u32 = 0x8569_AB44;
const TRAP_BOUNTY: u32 = 0x3233_F0A4;
/// The messages of the footer buttons: Race Stats and Restart (`PAD_BUTTON4` and `PAD_BUTTON5`).
const STATS: u32 = PAD_BUTTON0 + 4;
const RESTART: u32 = PAD_BUTTON0 + 5;
/// The same buttons as pad bits: the second button (key 2) and key 1.
const STATS_BIT: u32 = pad::BUTTON0 << 4;
const RESTART_BITS: u32 = (pad::BUTTON0 << 1) | (pad::BUTTON0 << 5);

/// Shows a [`PhotoReport`], with the race's report behind its Race Stats button.
pub struct PhotoLogic {
    photo: PhotoReport,
    report: Option<RaceReport>,
    /// The group has been faded in: the package's own first script rests it hidden, so the fade starts after the first
    /// frame.
    faded: bool,
    /// The picture is swapped for the race's backdrop and shown (the install has the texture).
    picture: bool,
    /// The buttons that already acted in this frame, as pad bits: a press reaches the screen as a pad bit and as the package's
    /// message, and must act once.
    done: u32,
}

impl PhotoLogic {
    /// The photo of a won race, with the lap table behind Race Stats.
    pub fn for_race(photo: PhotoReport, report: RaceReport) -> Self {
        Self { photo, report: Some(report), faded: false, picture: false, done: 0 }
    }

    /// The photo of a speed trap: Continue only.
    pub fn for_trap(photo: PhotoReport) -> Self {
        Self { photo, report: None, faded: false, picture: false, done: 0 }
    }

    /// Race Stats: the results, over the photo (their Back returns to it). Once per press.
    fn show_stats(&mut self, cx: &mut Cx) {
        let Some(report) = self.report.clone() else { return };
        if self.once(STATS_BIT) {
            cx.send(Command::ShowResults(report));
        }
    }

    /// Restart: the in-race question. Once per press.
    fn ask_restart(&mut self, cx: &mut Cx) {
        if self.report.is_some() && self.once(RESTART_BITS) {
            cx.send(Command::Confirm(Confirm::restart_results()));
        }
    }

    /// Whether the button `bit` has not acted yet in this frame; the first ask takes it (`done`).
    fn once(&mut self, bit: u32) -> bool {
        let first = self.done & bit == 0;
        self.done |= bit;
        first
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
        let cash = format!("{} {}", i18n::game_or_error(CASH_WORD), p.cash);
        match p.kind {
            PhotoKind::Sprint => {
                cx.text_object(SPRINT_CASH, &cash);
                cx.text_object(SPRINT_TIME, &format!("{} @ {}", clock(p.time), speed(p.speed, units)));
            }
            PhotoKind::Drag => {
                cx.text_object(RIVAL_CASH, &cash);
                let secs = format!("{:.1}", p.time);
                let time = t!("results-photo-drag-time", secs = secs, speed = speed(p.speed, units));
                cx.text_object(RIVAL_TIME, &time);
            }
            PhotoKind::Tollbooth => {
                cx.text_object(TOLL_SPEED, &speed(p.speed, units));
                cx.text_object(TOLL_TIME, &clock(p.time_left));
                cx.text_object(TOLL_CASH, &cash);
                cx.text_object(TOLL_BONUS, &format!("+{}", clock_short(p.bonus_secs)));
            }
            PhotoKind::SpeedTrap => {
                cx.text_object(TRAP_SPEED, &speed(p.speed, units));
                let bounty = grouped(i64::from(p.bounty));
                let line = blackbox_text::printf::fill(&i18n::game_or_error(BOUNTY_TEMPLATE), &[bounty.as_str()]);
                cx.text_object(TRAP_BOUNTY, &line);
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

    /// Swaps the package's picture for the race's backdrop (a polaroid with a transparent window, from
    /// `HUDTEXTURESPHOTOFINISH.BIN`, which the UI assets hold). Without the texture the picture stays hidden and the
    /// frozen world is shown whole.
    fn set_picture(&mut self, cx: &mut Cx) {
        let Some(name) = self.photo.picture else { return };
        let Some(picture) = cx.object(PICTURE) else { return };
        let key = fe_hash_upper(name);
        if cx.assets.texture(key).is_none() {
            log::warn!("photo: the backdrop {name} is not in the UI assets");
            return;
        }
        cx.rt.set_texture(picture, key);
        self.picture = true;
    }

    /// Keeps every other group of the package hidden, and the picture hidden unless it holds the race's backdrop.
    fn hold(&self, cx: &mut Cx) {
        let shown = self.group();
        for group in [SPRINT_GROUP, RIVAL_GROUP, TOLL_BOOTH_GROUP, SPEEDTRAP_GROUP] {
            cx.hide(group, group != shown);
        }
        cx.hide(CAMERA_FRAME, self.photo.kind != PhotoKind::SpeedTrap);
        cx.hide(PICTURE, !self.picture);
        if let (true, Some(picture)) = (self.picture, cx.object(PICTURE)) {
            cx.rt.set_alpha(picture, 255);
        }
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

/// A speed in the player's unit, with the unit's word.
fn speed(mps: f32, units: SpeedUnits) -> String {
    let unit = match units {
        SpeedUnits::Metric => KPH_LABEL,
        SpeedUnits::Imperial => MPH_LABEL,
    };
    format!("{} {}", speed_number(mps, units), i18n::game_or_error(unit))
}

/// The number of a speed in the player's unit, whole.
fn speed_number(mps: f32, units: SpeedUnits) -> String {
    match units {
        SpeedUnits::Metric => format!("{:.0}", mps * 3.6),
        SpeedUnits::Imperial => format!("{:.0}", mps * 2.236_936),
    }
}

impl ScreenLogic for PhotoLogic {
    fn start(&mut self, cx: &mut Cx) {
        self.fill(cx);
        self.set_picture(cx);
        self.hold(cx);
    }

    fn tick(&mut self, cx: &mut Cx, _dt: f32) {
        self.done = 0;
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
        match (message, self.report.is_some()) {
            (PAD_ACCEPT, true) => cx.send(Command::RaceAnswer(RaceAnswer::Exit)),
            (PAD_ACCEPT, false) => cx.send(Command::Resume),
            (STATS, true) => self.show_stats(cx),
            (RESTART, true) => self.ask_restart(cx),
            _ => {}
        }
    }

    /// The footer's Race Stats and Restart buttons answer the pad's second button and key 1 whichever footer button has the
    /// focus, as the results screen's do; the package's own message for the same press is passed over.
    fn pad_pressed(&mut self, cx: &mut Cx, bits: u32) {
        if bits & STATS_BIT != 0 {
            self.show_stats(cx);
        }
        if bits & RESTART_BITS != 0 {
            self.ask_restart(cx);
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
        assert_eq!(speed_number(50.0, SpeedUnits::Metric), "180");
        assert_eq!(speed_number(50.0, SpeedUnits::Imperial), "112");
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
