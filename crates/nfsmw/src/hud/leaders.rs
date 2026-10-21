//! The race leaderboard of `HUD_SingleRace.fng` (`LeaderBoardGroup`): what it shows and when, as plain data. The
//! package binding is `hud/leaderboard.rs`; the rules are in `docs/specs/race-mode.md` section 10 and the reading in
//! `docs/provenance/race-leaderboard.md`.

/// The rows of the board: the four best placed racers.
pub const ROWS: usize = 4;

/// Updates between two swaps of the rows' text (the original counts 90 of its HUD updates), at its 60 Hz **[unconfirmed]**.
const PERIOD_TICKS: i32 = 90;
const TICK_SECS: f32 = 1.0 / 60.0;

/// Language labels of the units and states (`English.bin`: `FT`, `M`, `KM/H`, `MPH`, `BUSTED`, `K.O.`).
pub const LABEL_FEET: u32 = 0x2E84_96B1;
pub const LABEL_METRES: u32 = 0xE207_8322;
pub const LABEL_KMH: u32 = 0x8569_A25F;
pub const LABEL_MPH: u32 = 0x8569_AB44;
pub const LABEL_BUSTED: u32 = 0x532B_5186;
pub const LABEL_KNOCKED_OUT: u32 = 0x5D82_DBA2;

/// The constants of the original's gap formula: a percent is a hundredth of the race, a metre is 3.28084 feet, and a
/// km/h is 0.27778 m/s and then 2.23694 mph.
const FEET_PER_METRE: f32 = 3.280_84;
const MS_PER_KMH: f32 = 0.277_78;
const MPH_PER_MS: f32 = 2.236_94;

/// One racer on the board.
#[derive(Debug, Clone, PartialEq)]
pub struct LeaderRow {
    pub name: String,
    /// Percent of the race done, 0 to 100 (100 once finished).
    pub percent: f32,
    /// The speed-trap points, in km/h (a speed-trap race shows them in the place of the gap).
    pub points: f32,
    /// Knocked out or totalled: the row says `K.O.`.
    pub out: bool,
    /// Busted: the row says `BUSTED`.
    pub busted: bool,
}

/// A racer before the board picks its rows.
#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    /// The racer's place now, 1 for first.
    pub place: u32,
    pub player: bool,
    pub row: LeaderRow,
}

/// What the board shows: the best four racers in place order, and the figures the gaps are measured with.
#[derive(Debug, Clone, PartialEq)]
pub struct Leaders {
    /// The four best placed racers (fewer in a small race), first place first.
    pub rows: Vec<LeaderRow>,
    /// All the racers of the race: the board is hidden below two.
    pub racers: u32,
    /// The row of the player, `None` when the player is not among the four.
    pub player: Option<usize>,
    /// The player's percent, which every gap is measured from (also when the player is not among the rows).
    pub player_percent: f32,
    /// The length of the whole race, metres.
    pub length: f32,
    /// A speed-trap race: the rows say the points, not the gap.
    pub speed_trap: bool,
}

impl Leaders {
    /// Picks the rows from every racer of the race.
    pub fn pick(mut entries: Vec<Entry>, length: f32, speed_trap: bool) -> Self {
        let racers = entries.len() as u32;
        entries.sort_by_key(|e| e.place);
        let player_percent = entries.iter().find(|e| e.player).map_or(0.0, |e| e.row.percent);
        let player = entries.iter().take(ROWS).position(|e| e.player);
        let rows = entries.into_iter().take(ROWS).map(|e| e.row).collect();
        Self { rows, racers, player, player_percent, length, speed_trap }
    }

    /// How many rows show: none for a race of one racer (the original hides the rows from the racer count on, and a
    /// single racer has nobody to compare with).
    pub fn shown_rows(&self) -> usize {
        match self.racers {
            0 | 1 => 0,
            n => (n as usize).min(ROWS).min(self.rows.len()),
        }
    }
}

/// The unit and state words the board prints, from the language table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Words {
    pub feet: String,
    pub metres: String,
    pub kmh: String,
    pub mph: String,
    pub busted: String,
    pub knocked_out: String,
}

impl Default for Words {
    /// The English words, for a run with no language table.
    fn default() -> Self {
        Self {
            feet: "FT".into(),
            metres: "M".into(),
            kmh: "KM/H".into(),
            mph: "MPH".into(),
            busted: "BUSTED".into(),
            knocked_out: "K.O.".into(),
        }
    }
}

/// An integer with the locale's digit grouping (`%$d`, `%$0.0f`): commas in English.
pub fn group_digits(value: f64) -> String {
    let text = format!("{:.0}", value.abs());
    let mut out = String::new();
    for (i, digit) in text.chars().enumerate() {
        if i > 0 && (text.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(digit);
    }
    out
}

impl Leaders {
    /// The text of row `row` in the gap phase: a state word for a racer that is out, the points of a speed-trap race,
    /// `-----` for the player, else the gap to the player in feet or metres with its sign (`-609 FT`: the racer is 609 feet
    /// behind the player; `+` ahead) **[exe]**.
    pub fn status(&self, row: usize, metric: bool, words: &Words) -> String {
        let Some(r) = self.rows.get(row) else { return String::new() };
        if r.busted {
            return words.busted.clone();
        }
        if r.out {
            return words.knocked_out.clone();
        }
        if self.speed_trap {
            let (value, unit) = match metric {
                true => (r.points, &words.kmh),
                false => (r.points * MS_PER_KMH * MPH_PER_MS, &words.mph),
            };
            return format!("{} {unit}", group_digits(f64::from(value)));
        }
        if self.player == Some(row) {
            return "-----".into();
        }
        let behind = r.percent < self.player_percent;
        let sign = if behind { '-' } else { '+' };
        let metres = (r.percent - self.player_percent).abs() * 0.01 * self.length;
        let (value, unit) = match metric {
            true => (metres, &words.metres),
            false => (metres * FEET_PER_METRE, &words.feet),
        };
        format!("{sign}{} {unit}", group_digits(f64::from(value)))
    }
}

/// Which text the rows show: the racers' names with their place, then their gaps.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Names,
    Gaps,
}

/// The swap between the two phases. A countdown of 90 updates ends in the rows leaving (script `LEAVE`); once every row's
/// `LEAVE` has played, the rows enter again (script `ENTER`) with the other phase's text and the countdown starts anew.
#[derive(Debug, Clone, PartialEq)]
pub struct Rotation {
    counter: i32,
    phase: Phase,
    leaving: bool,
    carry: f32,
}

impl Default for Rotation {
    fn default() -> Self {
        Self { counter: PERIOD_TICKS, phase: Phase::Names, leaving: false, carry: 0.0 }
    }
}

impl Rotation {
    pub fn phase(&self) -> Phase {
        self.phase
    }

    /// The rows are leaving: waiting for their `LEAVE` scripts to end.
    pub fn leaving(&self) -> bool {
        self.leaving
    }

    /// Counts `dt` seconds of updates down.
    pub fn advance(&mut self, dt: f32) {
        self.carry += dt.max(0.0);
        let ticks = (self.carry / TICK_SECS).floor();
        self.carry -= ticks * TICK_SECS;
        self.counter = (self.counter - ticks as i32).max(-PERIOD_TICKS);
    }

    /// The countdown is over and the rows have not yet left: they leave now. Once only per swap.
    pub fn take_leave(&mut self) -> bool {
        if self.counter > 0 || self.leaving {
            return false;
        }
        self.leaving = true;
        true
    }

    /// The rows have entered again: the other phase's text, and the countdown starts anew.
    pub fn swap(&mut self) {
        self.counter = PERIOD_TICKS;
        self.leaving = false;
        self.phase = match self.phase {
            Phase::Names => Phase::Gaps,
            Phase::Gaps => Phase::Names,
        };
    }
}
