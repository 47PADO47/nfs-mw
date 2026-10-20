//! The race readout of `HUD_SingleRace.fng`: the place, the lap, the clock, the progress and the 3-2-1-GO group.
//! What each object is, and which names are not in the package, is in `docs/provenance/race-mode.md` section 9.

use blackbox_feng::{ObjectRef, PackageId, Runtime, fe_hash_upper};

/// The race information group: the position, time and lap backings, the clock and the strings on them.
const INFO_GROUP: u32 = 0xB1C5_ED0D;
/// The clock (`CurrentLapTime`) and its icon.
const CLOCK: &str = "CurrentLapTime";
const CLOCK_ICON: &str = "TIMER_ICON";
/// The place group (`POSITION_GROUP`): the place string above the number of racers.
const POSITION_GROUP: &str = "POSITION_GROUP";
const PLACE: u32 = 0x9C18_3BF8;
const RACERS: u32 = 0x3BBD_6268;
/// The lap group: the lap now and the laps in all.
const LAP_GROUP: u32 = 0x1A09_1045;
const LAP_NOW: u32 = 0x1CDD_D8D0;
const LAP_TOTAL: u32 = 0x18A7_ACD2;
/// The progress group: the percentage and `COMPLETE`.
const PROGRESS_GROUP: u32 = 0xE3DD_CD09;
const PROGRESS: u32 = 0x9CB5_C95D;
/// The countdown group and the two strings of its text (the shadow and the face).
const COUNT_GROUP: &str = "321_GO_GROUP";
const COUNT_TEXT: [u32; 2] = [0xB969_E9EF, 0x2DA2_20AA];

/// Where the countdown is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Count {
    Three,
    Two,
    One,
    Go,
}

impl Count {
    /// The script of the group that plays the step (`THREE`, `TWO`, `ONE`, `GO`).
    fn script(self) -> u32 {
        fe_hash_upper(match self {
            Count::Three => "THREE",
            Count::Two => "TWO",
            Count::One => "ONE",
            Count::Go => "GO",
        })
    }

    fn text(self) -> &'static str {
        match self {
            Count::Three => "3",
            Count::Two => "2",
            Count::One => "1",
            Count::Go => "Go!",
        }
    }

    /// The step a banner text names (`3`, `2`, `1`, `GO`).
    pub fn from_banner(text: &str) -> Option<Self> {
        match text {
            "3" => Some(Count::Three),
            "2" => Some(Count::Two),
            "1" => Some(Count::One),
            "GO" => Some(Count::Go),
            _ => None,
        }
    }
}

/// A line the HUD shows for a few seconds (the generic message group, `Hud_GenericMessage`): a checkpoint, a lap, a bonus,
/// the race-over message. A new `serial` starts the line again.
#[derive(Debug, Clone, PartialEq)]
pub struct Message {
    pub serial: u32,
    pub text: String,
}

/// The generic message group (`0xE0BA07EC`, its child `0x32A7A521` plays the scripts) and its two strings, the shadow and
/// the face, both named `0xDA768EA6` (so all objects of the name are taken).
const MESSAGE_GROUP: u32 = 0xE0BA_07EC;
const MESSAGE_PLAYER: u32 = 0x32A7_A521;
const MESSAGE_TEXT: u32 = 0xDA76_8EA6;
/// The script of the group that fades the line in and out: 3200 ms, four colour keys (the length of the others is 2400,
/// 3200, 3600 and 1200; which one the game picks for a kind of line is not read) **[unconfirmed]**.
const MESSAGE_SCRIPT: u32 = 0xA19B_B14C;
const MESSAGE_SECS: f32 = 3.2;

/// What the race part of the HUD shows. `None` in `HudState` means no race.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RaceHud {
    /// The player's place, 1 for first, among `racers`.
    pub place: u32,
    pub racers: u32,
    /// The lap now and the laps in all; `None` for a sprint.
    pub lap: Option<(u32, u32)>,
    /// The seconds on the clock: the time left under a time limit, else the time since GO.
    pub clock: f32,
    /// How much of the course is done, 0 to 1.
    pub progress: f32,
    /// The countdown step now, while the group is up.
    pub count: Option<Count>,
}

/// The name hashes of the roots the race part shows.
pub(super) fn root_hashes() -> [u32; 4] {
    [INFO_GROUP, fe_hash_upper(CLOCK), fe_hash_upper(CLOCK_ICON), fe_hash_upper(POSITION_GROUP)]
}

/// The countdown group's name hash: it shows only while a step is up.
pub(super) fn message_hash() -> u32 {
    MESSAGE_GROUP
}

pub(super) fn count_hash() -> u32 {
    fe_hash_upper(COUNT_GROUP)
}

/// The lap and progress groups: shown for a lapped race, and for a sprint, in that order.
pub(super) fn lap_hash() -> u32 {
    LAP_GROUP
}

pub(super) fn progress_hash() -> u32 {
    PROGRESS_GROUP
}

/// Every object of the package with this name hash: two strings can share a name (a shadow and its face).
fn named_all(rt: &Runtime, package: PackageId, name_hash: u32) -> Vec<ObjectRef> {
    let Some(def) = rt.package(package) else { return Vec::new() };
    let guids = def.objects.iter().filter(|o| o.name_hash == name_hash).map(|o| o.guid);
    guids.filter_map(|guid| rt.find_guid(package, guid)).collect()
}

/// The objects of the race part, found once.
pub(super) struct Binding {
    place: Option<ObjectRef>,
    racers: Option<ObjectRef>,
    /// The lap and progress groups: their authored fade-in is a script on a message this build does not raise, so
    /// they are held at full alpha while a race runs (as the pursuit board is).
    frames: Vec<ObjectRef>,
    lap_now: Option<ObjectRef>,
    lap_total: Option<ObjectRef>,
    progress: Option<ObjectRef>,
    clock: Option<ObjectRef>,
    count_group: Option<ObjectRef>,
    count_text: Vec<ObjectRef>,
    shown_count: Option<Count>,
    message_group: Option<ObjectRef>,
    message_player: Option<ObjectRef>,
    /// The strings of the line (the object table keeps one per name, so both are found by their guid in the package).
    message_text: Vec<ObjectRef>,
    shown_serial: Option<u32>,
    message_left: f32,
}

impl Binding {
    pub fn new(rt: &Runtime, package: PackageId) -> Self {
        Self {
            place: rt.find(package, PLACE),
            racers: rt.find(package, RACERS),
            frames: [LAP_GROUP, PROGRESS_GROUP].iter().filter_map(|h| rt.find(package, *h)).collect(),
            lap_now: rt.find(package, LAP_NOW),
            lap_total: rt.find(package, LAP_TOTAL),
            progress: rt.find(package, PROGRESS),
            clock: rt.find(package, fe_hash_upper(CLOCK)),
            count_group: rt.find(package, fe_hash_upper(COUNT_GROUP)),
            count_text: COUNT_TEXT.iter().filter_map(|h| rt.find(package, *h)).collect(),
            shown_count: None,
            message_group: rt.find(package, MESSAGE_GROUP),
            message_player: rt.find(package, MESSAGE_PLAYER),
            message_text: named_all(rt, package, MESSAGE_TEXT),
            shown_serial: None,
            message_left: 0.0,
        }
    }

    /// Counts the line's time down.
    pub fn advance(&mut self, dt: f32) {
        self.message_left = (self.message_left - dt).max(0.0);
    }

    /// The line is on: its group has to show.
    pub fn message_active(&self) -> bool {
        self.message_left > 0.0
    }

    /// Puts a line up when its serial is new, and keeps the group at full alpha while it plays.
    pub fn show_message(&mut self, rt: &mut Runtime, message: Option<&Message>) {
        let Some(message) = message else { return };
        if let Some(group) = self.message_group {
            rt.set_alpha(group, 255);
        }
        if self.shown_serial == Some(message.serial) {
            return;
        }
        self.shown_serial = Some(message.serial);
        self.message_left = MESSAGE_SECS;
        for text in &self.message_text {
            rt.set_text(*text, &message.text);
        }
        if let Some(player) = self.message_player {
            rt.run_script(player, MESSAGE_SCRIPT);
        }
    }

    pub fn apply(&mut self, rt: &mut Runtime, race: Option<RaceHud>) {
        let Some(r) = race else {
            self.shown_count = None;
            return;
        };
        for frame in &self.frames {
            rt.set_alpha(*frame, 255);
        }
        if let Some(o) = self.place {
            rt.set_text(o, r.place.to_string());
        }
        if let Some(o) = self.racers {
            rt.set_text(o, r.racers.to_string());
        }
        if let Some((now, total)) = r.lap {
            if let Some(o) = self.lap_now {
                rt.set_text(o, now.to_string());
            }
            if let Some(o) = self.lap_total {
                rt.set_text(o, total.to_string());
            }
        }
        if let Some(o) = self.progress {
            rt.set_text(o, format!("{}% ", (r.progress.clamp(0.0, 1.0) * 100.0) as u32));
        }
        if let Some(o) = self.clock {
            rt.set_text(o, super::pursuit::timer_text(r.clock));
        }
        self.count(rt, r.count);
    }

    /// Starts the countdown step's script on its group when the step changes.
    fn count(&mut self, rt: &mut Runtime, count: Option<Count>) {
        if count == self.shown_count {
            return;
        }
        self.shown_count = count;
        let (Some(step), Some(group)) = (count, self.count_group) else { return };
        for text in &self.count_text {
            rt.set_text(*text, step.text());
        }
        rt.run_script(group, step.script());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_banner_names_the_countdown_steps() {
        assert_eq!(Count::from_banner("3"), Some(Count::Three));
        assert_eq!(Count::from_banner("GO"), Some(Count::Go));
        assert_eq!(Count::from_banner("Finished"), None);
    }

    #[test]
    fn the_scripts_are_the_package_ids() {
        assert_eq!(Count::Three.script(), 0x03c3_c2f7);
        assert_eq!(Count::Go.script(), 0x535);
    }
}
