//! The race leaderboard in the package: the group `LeaderBoardGroup` of `HUD_SingleRace.fng` with its four rows
//! (`LBData_k` the value, `LeaderText_k` the place, `OL_ICON_k` the online headset, `LBBacking_k` the backing). What a row
//! says is `hud/leaders.rs`; the reading of the original's update is `docs/provenance/race-leaderboard.md`.

use blackbox_feng::{ObjectRef, PackageId, Runtime};

use super::leaders::{
    LABEL_BUSTED, LABEL_FEET, LABEL_KMH, LABEL_KNOCKED_OUT, LABEL_METRES, LABEL_MPH, Leaders, Phase, ROWS, Rotation,
    Words,
};
use super::state::HudState;

/// The group, and the four objects of each row (the name hash of row 1; the others follow by one).
pub(super) const GROUP: u32 = 0x5ED6_09A1;
const DATA: u32 = 0x2AF8_9FD7;
const PLACE: u32 = 0x4183_E0A1;
const ICON: u32 = 0x51F5_B673;
const BACKING: u32 = 0x4FDD_D64C;

/// The scripts of a row: the strings leave (`LEAVE`) and enter (`ENTER`) when the text swaps, the backing of the player's
/// row plays `HIGHLIGHT` and the others rest in `INIT`.
const LEAVE: u32 = 0x0331_13AC;
const ENTER: u32 = 0x5079_C8F8;
const HIGHLIGHT: u32 = 0x249D_B7B7;
const INIT: u32 = blackbox_feng::runtime::INIT_SCRIPT;

/// The name hashes of one kind of row object, for the layout (`hud/elements.rs`): row `k` is `base + k`.
pub(super) fn row_hashes(kind: Kind) -> [u32; ROWS] {
    let base = match kind {
        Kind::Data => DATA,
        Kind::Place => PLACE,
        Kind::Icon => ICON,
        Kind::Backing => BACKING,
    };
    std::array::from_fn(|k| base + k as u32)
}

#[derive(Clone, Copy)]
pub(super) enum Kind {
    Data,
    Place,
    Icon,
    Backing,
}

/// The objects of the board, found once.
pub(super) struct Binding {
    group: Option<ObjectRef>,
    data: [Option<ObjectRef>; ROWS],
    place: [Option<ObjectRef>; ROWS],
    backing: [Option<ObjectRef>; ROWS],
    rotation: Rotation,
    /// The rows have shown a race: their scripts are put back when it ends.
    used: bool,
    /// The unit words, read from the language table once.
    words: Option<Words>,
}

fn find_all(rt: &Runtime, package: PackageId, kind: Kind) -> [Option<ObjectRef>; ROWS] {
    row_hashes(kind).map(|hash| rt.find(package, hash))
}

impl Binding {
    pub fn new(rt: &Runtime, package: PackageId) -> Self {
        Self {
            group: rt.find(package, GROUP),
            data: find_all(rt, package, Kind::Data),
            place: find_all(rt, package, Kind::Place),
            backing: find_all(rt, package, Kind::Backing),
            rotation: Rotation::default(),
            used: false,
            words: None,
        }
    }

    /// Counts the swap's countdown while a race shows the board.
    pub fn advance(&mut self, dt: f32) {
        self.rotation.advance(dt);
    }

    pub fn apply(&mut self, rt: &mut Runtime, s: &HudState) {
        let rows = s.leaders.as_ref().map_or(0, Leaders::shown_rows);
        let Some(leaders) = s.leaders.as_ref().filter(|_| rows > 0) else {
            self.rest(rt);
            return;
        };
        self.used = true;
        if let Some(group) = self.group {
            ensure(rt, group, INIT);
        }
        self.swap_rows(rt, rows);
        let words = self.words.get_or_insert_with(|| words_of(rt));
        let metric = !s.use_mph;
        for k in 0..rows {
            let Some(data) = self.data[k] else { continue };
            match self.rotation.phase() {
                Phase::Names => {
                    rt.set_text(data, leaders.rows[k].name.clone());
                    if let Some(place) = self.place[k] {
                        rt.set_text(place, (k + 1).to_string());
                    }
                }
                Phase::Gaps => rt.set_text(data, leaders.status(k, metric, words)),
            }
        }
        for (k, backing) in self.backing.iter().enumerate() {
            let Some(backing) = *backing else { continue };
            let script = if leaders.player == Some(k) { HIGHLIGHT } else { INIT };
            ensure(rt, backing, script);
        }
    }

    /// The swap of the text: when the countdown is over the rows leave; once their `LEAVE` has played they enter again
    /// with the other phase's text **[exe]**.
    fn swap_rows(&mut self, rt: &mut Runtime, rows: usize) {
        if self.rotation.take_leave() {
            for data in self.data.iter().take(rows).flatten() {
                rt.run_script(*data, LEAVE);
            }
            return;
        }
        if !self.rotation.leaving() {
            return;
        }
        let mut entered = false;
        for data in self.data.iter().take(rows).flatten() {
            let script = rt.script_of(*data);
            if script != Some(LEAVE) || !ended(rt, *data) {
                continue;
            }
            rt.run_script(*data, ENTER);
            entered = true;
        }
        if entered {
            self.rotation.swap();
        }
    }

    /// No race shows the board: the countdown starts anew and the rows rest in their start script for the next race.
    fn rest(&mut self, rt: &mut Runtime) {
        if !self.used {
            return;
        }
        self.used = false;
        self.rotation = Rotation::default();
        for object in self.data.iter().chain(&self.place).flatten() {
            ensure(rt, *object, INIT);
        }
    }
}

/// Starts `script` on `o` unless it is the one playing.
fn ensure(rt: &mut Runtime, o: ObjectRef, script: u32) {
    if rt.script_of(o) != Some(script) {
        rt.run_script(o, script);
    }
}

/// The object's script has played to its end (a script that runs once stops one tick past its length).
fn ended(rt: &Runtime, o: ObjectRef) -> bool {
    let Some(state) = rt.object(o) else { return false };
    let Some(current) = state.current else { return false };
    state.time > state.scripts[current].length
}

/// The words from the language table, with the English ones where it has none.
fn words_of(rt: &Runtime) -> Words {
    let d = Words::default();
    let word = |label: u32, default: String| rt.label_text(label).unwrap_or(default);
    Words {
        feet: word(LABEL_FEET, d.feet),
        metres: word(LABEL_METRES, d.metres),
        kmh: word(LABEL_KMH, d.kmh),
        mph: word(LABEL_MPH, d.mph),
        busted: word(LABEL_BUSTED, d.busted),
        knocked_out: word(LABEL_KNOCKED_OUT, d.knocked_out),
    }
}
