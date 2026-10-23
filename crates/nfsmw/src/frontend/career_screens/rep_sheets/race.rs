//! The race events sheet (`SafeHouseRaceSheet.fng`): the free races of the rival's bin, then its boss races.
//!
//! A race is open once its bin has been entered (a boss race once the rival is unlocked), and done once it is won in the
//! career (`career-blacklist.md` sections 2.6 and 4.1). Accept asks `REP_SHEET_RACE_CONFIRM`; on Yes the career race
//! starts like a rival challenge does ([`Command::LaunchEvent`]).

use blackbox_text::label_hash;
use nfsmw_career::blacklist::{BinRecord, BinState, Blacklist};
use nfsmw_data::gameplay::{Race, RaceType};

use super::{Ask, Page, Status};
use crate::career::{CareerHandle, race_event_id};
use crate::frontend::challenge_series::Crop;
use crate::frontend::custom_race::records;
use crate::frontend::logic::{Command, Cx};
use crate::frontend::quick_play::text;
use crate::frontend::quick_race::{self, Board};
use crate::settings::SpeedUnits;
use crate::ui::{Text, UiAssets};

/// Objects of `SafeHouseRaceSheet.fng` by name hash.
const KIND_NAME: u32 = 0x9BA7_8BA2;
const KIND_ICON: u32 = 0xF97E_C5D5;
const TRACK_NAME: u32 = 0x0F2C_D475;
/// The police line (label and value): the original derives the value from data that was not read, so it is hidden.
const POLICE: [u32; 2] = [0x9CA1_1B4A, 0x0000_9B21];
const BEST: u32 = 0x8FD4_1BB4;
/// The groups of the average and the top speed: a race keeps neither, so they are hidden.
const SPEEDS: [u32; 2] = [0x6716_A779, 0xB0F9_8EEE];
const CASH: [u32; 2] = [0xFFBB_BF24, 0x03A8_0495];
const DISTANCE: u32 = 0x080C_9DAA;
const LAPS_GROUP: u32 = 0xEDE0_7C9B;
const LAPS: u32 = 0x0018_B36F;
const UNLOCK_LABEL: u32 = 0xBD56_3BE5;

/// Every object of the package the page names.
#[cfg(test)]
pub(super) fn used() -> Vec<u32> {
    let mut all = vec![KIND_NAME, KIND_ICON, TRACK_NAME, BEST, DISTANCE, LAPS_GROUP, LAPS];
    all.extend(POLICE);
    all.extend(SPEEDS);
    all.extend(CASH);
    all
}

/// One race of the sheet.
#[derive(Clone, Debug, PartialEq)]
pub struct RaceRow {
    /// The event id (`10.2.1`), which names the race to the career and to the language table.
    pub event: String,
    pub kind: RaceType,
    pub boss: bool,
    pub status: Status,
    /// The cash a win pays.
    pub cash: u32,
    /// The laps the race runs, at least one.
    pub laps: u32,
    /// The route in metres, 0 when the race has no length.
    pub length: f32,
    /// The picture of the race on the city map and the part of it to show (0 and the whole picture without a track).
    pub map: u32,
    pub crop: Crop,
}

/// The rows of bin `bin`: its free races, then its boss races. A race the install has no data for is left out.
pub fn race_rows(blacklist: &Blacklist, races: &[Race], board: Option<&Board>, bin: u8) -> Vec<RaceRow> {
    let Some(record) = blacklist.bin(bin) else { return Vec::new() };
    let free = record.def.races.iter().map(|event| (event, false));
    let boss = record.def.rival_events.iter().map(|event| (event, true));
    free.chain(boss).filter_map(|(event, boss)| row(record, races, board, event, boss)).collect()
}

fn row(record: &BinRecord, races: &[Race], board: Option<&Board>, event: &str, boss: bool) -> Option<RaceRow> {
    let race = races.iter().find(|race| race_event_id(race) == event)?;
    let track = board.and_then(|board| board.tracks.iter().find(|track| track.event == event));
    Some(RaceRow {
        event: event.to_owned(),
        kind: race.race_type.unwrap_or(RaceType::Sprint),
        boss,
        status: status_of(record, event, boss),
        cash: race.params.cash_value.max(0.0) as u32,
        laps: u32::try_from(race.laps).unwrap_or(1).max(1),
        length: track.map_or(0.0, |track| track.brief_length()),
        map: track.map_or(0, |track| track.map_texture),
        crop: track.map_or(Crop::WHOLE, |track| track.crop),
    })
}

/// Where a race stands: won, open or locked.
pub fn status_of(record: &BinRecord, event: &str, boss: bool) -> Status {
    let won = if boss { &record.rival_races_won } else { &record.races_won };
    if won.iter().any(|done| done == event) {
        return Status::Done;
    }
    let open = match (boss, record.state) {
        (_, None) => false,
        (false, Some(_)) => true,
        (true, Some(state)) => state != BinState::RivalLocked,
    };
    match open {
        true => Status::Open,
        false => Status::Locked,
    }
}

/// The rows of the race events sheet.
pub struct RacePage {
    bin: u8,
    rows: Vec<RaceRow>,
}

impl RacePage {
    /// The page of bin `bin` from the career being played.
    pub fn read(bin: u8) -> RacePage {
        let board = quick_race::board();
        let rows = CareerHandle::shared()
            .with(|career| race_rows(&career.blacklist, &career.data.gameplay.races, board.as_deref(), bin))
            .unwrap_or_default();
        RacePage { bin, rows }
    }
}

/// The text of a game label, empty when it is missing.
fn game_text(assets: &UiAssets, label: u32) -> String {
    Text::Game(label).resolve(assets).unwrap_or_default()
}

fn track_name(assets: &UiAssets, event: &str) -> String {
    Text::Game(label_hash(&text::track_label_key(event))).resolve(assets).unwrap_or_else(|| event.to_owned())
}

impl Page for RacePage {
    fn len(&self) -> usize {
        self.rows.len()
    }

    fn status(&self, at: usize) -> Status {
        self.rows.get(at).map_or(Status::Locked, |row| row.status)
    }

    fn name(&self, assets: &UiAssets, at: usize) -> String {
        self.rows.get(at).map(|row| track_name(assets, &row.event)).unwrap_or_default()
    }

    fn summary(&self, assets: &UiAssets, at: usize) -> String {
        let Some(row) = self.rows.get(at) else { return String::new() };
        let kind = game_text(assets, text::mode_label(row.kind));
        match row.status {
            Status::Done => format!("{kind}  (won)"),
            Status::Open => kind,
            Status::Locked => unlock_sentence(assets, self.bin),
        }
    }

    fn heading(&self) -> &'static str {
        "Race Events"
    }

    fn fill(&self, cx: &mut Cx, at: usize) {
        let Some(row) = self.rows.get(at) else { return };
        let metric = cx.settings.speed_units == SpeedUnits::Metric;
        let unit = Text::Game(text::unit_label(metric)).resolve(cx.assets).unwrap_or_default();
        cx.text_object(KIND_NAME, &game_text(cx.assets, text::mode_label(row.kind)));
        if let Some(icon) = cx.object(KIND_ICON) {
            cx.rt.set_texture(icon, text::mode_icon(row.kind));
        }
        cx.text_object(TRACK_NAME, &track_name(cx.assets, &row.event));
        for hidden in POLICE.into_iter().chain(SPEEDS) {
            cx.hide(hidden, true);
        }
        // A race that is not open has no record to show.
        let best = match row.status {
            Status::Locked => "--".to_owned(),
            _ => records::time_text(records::best(&row.event).time),
        };
        cx.text_object(BEST, &best);
        for name in CASH {
            cx.text_object(name, &crate::frontend::infractions::money(row.cash));
        }
        cx.text_object(DISTANCE, &text::length_text(row.length, metric, &unit));
        let laps = matches!(row.kind, RaceType::Circuit | RaceType::Knockout);
        cx.hide(LAPS_GROUP, !laps);
        cx.text_object(LAPS, &row.laps.to_string());
    }

    fn ask(&self, at: usize) -> Option<Ask> {
        let row = self.rows.get(at).filter(|row| row.status != Status::Locked)?;
        Some(Ask {
            label: label_hash("REP_SHEET_RACE_CONFIRM"),
            then: Command::LaunchEvent { event: row.event.clone() },
        })
    }

    fn map(&self, at: usize) -> Option<(u32, Crop)> {
        self.rows.get(at).filter(|row| row.map != 0).map(|row| (row.map, row.crop))
    }

    fn map_textures(&self) -> Vec<u32> {
        self.rows.iter().map(|row| row.map).filter(|map| *map != 0).collect()
    }

    /// The sheet starts on the first race that is not won yet.
    fn first(&self) -> usize {
        self.rows.iter().position(|row| row.status == Status::Open).unwrap_or(0)
    }
}

/// `Unlock this track by beating <alias>, Blacklist member <bin>.`
fn unlock_sentence(assets: &UiAssets, bin: u8) -> String {
    let template = game_text(assets, UNLOCK_LABEL);
    let alias = game_text(assets, label_hash(&text::alias_key(bin)));
    text::fill_sentence(&template, &alias, bin)
}
