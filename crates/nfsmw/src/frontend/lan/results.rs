//! A session's race results (MP-70): the result record the server sends at the end of a race, as the LAN results page's
//! rows and as the `RaceReport` the original's post-race page (`PostRace_Results.fng`, `post_race/`) draws.

use nfsmw_data::gameplay::RaceType;
use nfsmw_net::client_session::LobbyView;
use nfsmw_net::message::{Mode, RESULT_BUSTED, RESULT_DNF, RaceResults, ResultRow};
use nfsmw_net::results::{FLAG_KNOCKED_OUT, FLAG_LEFT};

use super::menu::rows::Row;
use super::text::{ordinal, race_time, slot_name};
use crate::viewer::{RaceReport, ReportRacer};

/// The rows in finishing order (a row without a place last).
pub fn ordered(results: &RaceResults) -> Vec<ResultRow> {
    let mut rows: Vec<ResultRow> = results.rows.iter().copied().collect();
    rows.sort_by_key(|row| (row.place == 0, row.place, row.slot));
    rows
}

/// What a row's flags say: `DNF`, `busted`, `knocked out`, `left`.
pub fn flags_text(flags: u8) -> String {
    let words: Vec<&str> =
        [(RESULT_DNF, "DNF"), (RESULT_BUSTED, "busted"), (FLAG_KNOCKED_OUT, "knocked out"), (FLAG_LEFT, "left")]
            .into_iter()
            .filter(|(bit, _)| flags & bit != 0)
            .map(|(_, word)| word)
            .collect();
    words.join(", ")
}

/// The page's rows: `1st  Ann` and `1:23.456  best 0:40.100  DNF`.
pub fn rows(results: &RaceResults, lobby: Option<&LobbyView>) -> Vec<Row> {
    ordered(results)
        .iter()
        .map(|row| {
            let place = match row.place {
                0 => "-".to_owned(),
                place => ordinal(u32::from(place)),
            };
            let label = format!("{place}  {}", slot_name(lobby, u32::from(row.slot)));
            let mut value = race_time(row.time_ms);
            if row.best_lap_ms != 0 {
                value.push_str(&format!("  best {}", race_time(row.best_lap_ms)));
            }
            let flags = flags_text(row.flags);
            if !flags.is_empty() {
                value.push_str(&format!("  {flags}"));
            }
            Row::info(label, value)
        })
        .collect()
}

/// The report the original's post-race page draws: this client's row first (the page's player), then the others in
/// finishing order; the kind and laps from the session's settings.
#[allow(dead_code)] // for the session world's race end (`crate::net`), which opens `PostRace_Results.fng` with it
pub fn report(results: &RaceResults, lobby: &LobbyView) -> RaceReport {
    let mut rows = ordered(results);
    if let Some(at) = rows.iter().position(|row| Some(row.slot) == lobby.my_slot) {
        let mine = rows.remove(at);
        rows.insert(0, mine);
    }
    let winner = rows.iter().filter(|row| row.place == 1).map(|row| row.time_ms).next();
    let racers = rows
        .iter()
        .map(|row| {
            let finished = row.flags & RESULT_DNF == 0 && row.time_ms > 0;
            let time = finished.then_some(row.time_ms as f32 / 1000.0);
            let mut racer = ReportRacer::bare(&slot_name(Some(lobby), u32::from(row.slot)), u32::from(row.place), time);
            racer.time_behind =
                winner.filter(|_| finished && row.place > 1).map(|best| (row.time_ms - best) as f32 / 1000.0);
            racer.knocked_out = row.flags & FLAG_KNOCKED_OUT != 0;
            racer
        })
        .collect();
    let settings = lobby.settings.as_ref();
    RaceReport {
        racers,
        kind: race_type(settings.map_or(Mode::Sprint, |s| s.mode)),
        laps: settings.map_or(1, |s| u32::from(s.laps.max(1))),
        photo: None,
        pursuit: None,
        // A session race is not a career race: nothing is asked after a loss.
        retry: nfsmw_career::story::LossRule::Leave,
    }
}

fn race_type(mode: Mode) -> RaceType {
    match mode {
        Mode::Circuit => RaceType::Circuit,
        Mode::Knockout => RaceType::Knockout,
        Mode::Speedtrap => RaceType::SpeedTrap,
        Mode::Drag => RaceType::Drag,
        _ => RaceType::Sprint,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::lan::NetFrontend;
    use crate::frontend::lan::mock::MockNetFrontend;
    use nfsmw_net::message::{EventHead, List};

    fn results() -> RaceResults {
        let row = |slot: u8, place: u8, time_ms: u32, flags: u8| ResultRow {
            slot,
            entity_id: u16::from(slot),
            place,
            time_ms,
            best_lap_ms: time_ms / 3,
            flags,
        };
        RaceResults {
            head: EventHead::default(),
            rows: List::new(vec![row(0, 2, 95_000, 0), row(2, 0, 0, RESULT_DNF), row(1, 1, 90_500, 0)]).unwrap(),
        }
    }

    #[test]
    fn the_rows_are_in_finishing_order_with_times_and_flags() {
        let net = MockNetFrontend::in_lobby(&["Host", "Ann", "Bob"], 0);
        let rows = rows(&results(), net.lobby());
        let lines: Vec<(&str, &str)> = rows.iter().map(|r| (r.label.as_str(), r.value.as_str())).collect();
        assert_eq!(
            lines,
            [("1st  Ann", "1:30.500  best 0:30.166"), ("2nd  Host", "1:35.000  best 0:31.666"), ("-  Bob", "-  DNF")]
        );
        assert_eq!(flags_text(RESULT_BUSTED | FLAG_LEFT), "busted, left");
    }

    #[test]
    fn the_report_puts_this_player_first() {
        let mut net = MockNetFrontend::in_lobby(&["Host", "Ann", "Bob"], 0);
        if let Some(settings) = net.lobby.as_mut().and_then(|lobby| lobby.settings.as_mut()) {
            settings.mode = Mode::Circuit;
            settings.laps = 3;
        }
        let report = report(&results(), net.lobby().unwrap());
        let names: Vec<&str> = report.racers.iter().map(|r| r.name.as_str()).collect();
        assert_eq!(names, ["Host", "Ann", "Bob"]);
        assert_eq!((report.kind, report.laps), (RaceType::Circuit, 3));
        assert_eq!(report.racers[0].time, Some(95.0));
        assert_eq!(report.racers[0].time_behind, Some(4.5));
        assert_eq!(report.racers[2].time, None, "a DNF has no time");
    }
}
