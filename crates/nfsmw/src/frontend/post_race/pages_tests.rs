//! The pages' rows: the numbers of the original's screenshots, the order of the standings and the page cycle.

use nfsmw_data::gameplay::RaceType;

use super::pages::*;
use crate::settings::SpeedUnits;
use crate::viewer::RaceReport;

fn text(s: &str) -> Cell {
    Cell::Text(s.to_owned())
}

const DASHES: Cell = Cell::Label(0x0FC1_BF40);

#[test]
fn a_time_reads_like_the_games() {
    assert_eq!(race_time(30.36), "30.36");
    assert_eq!(race_time(95.19), "1:35.19");
    assert_eq!(race_time(172.51), "2:52.51");
    assert_eq!(race_time(0.0), "0.00");
    assert_eq!(race_time(3723.4), "1:02:03");
    assert_eq!(race_time(-2.0), "0.00");
}

#[test]
fn the_hundredths_are_cut_not_rounded() {
    assert_eq!(race_time(59.999), "59.99");
}

#[test]
fn the_second_button_walks_the_three_pages_and_back() {
    let mut page = Page::Standings;
    let mut seen = Vec::new();
    for _ in 0..4 {
        seen.push(page);
        page = page.next();
    }
    assert_eq!(seen, [Page::Standings, Page::Statistics, Page::Stages, Page::Standings]);
}

#[test]
fn the_standings_list_the_racers_by_place_with_their_times() {
    let mut report = RaceReport::sample();
    report.racers.reverse();
    let table = table(&report, Page::Standings, 0, SpeedUnits::Imperial);
    assert_eq!(table.headers, [Some(0x96B0_5F47), Some(0xCE67_8AD3), Some(0xB67D_A102)]);
    assert_eq!(table.rows[0], [text("1"), text("stock"), text("2:52.51")]);
    let places: Vec<&Cell> = table.rows.iter().map(|r| &r[0]).collect();
    assert_eq!(places, [&text("1"), &text("2"), &text("3"), &text("4")]);
    assert!(!table.filter, "the standings have no racer filter");
    assert_eq!(table.footer, 0xD0B8_AA33, "the footer names the next page: Statistics");
}

#[test]
fn a_racer_without_a_time_has_dashes_and_a_knocked_out_one_says_so() {
    let mut report = RaceReport::sample();
    report.racers[1].time = None;
    report.racers[2].time = None;
    report.racers[2].knocked_out = true;
    let table = table(&report, Page::Standings, 0, SpeedUnits::Imperial);
    assert_eq!(table.rows[1][2], DASHES);
    assert_eq!(table.rows[2][2], Cell::Label(0x5D82_DBA2));
}

#[test]
fn the_statistics_of_the_sprint_match_the_screenshot() {
    let report = RaceReport::sample();
    let table = table(&report, Page::Statistics, 0, SpeedUnits::Imperial);
    let mph = |n: &str| Cell::WithUnit(n.into(), 0x8569_AB44);
    assert_eq!(table.title, 0x4E70_6980);
    assert_eq!(table.footer, 0x1135_F776, "Stages");
    assert!(table.filter);
    assert_eq!(table.headers, [None; 3]);
    assert_eq!(
        table.rows,
        [
            [Cell::Label(0x0EF3_4382), Cell::Empty, mph("123")],
            [Cell::Label(0x57F4_140A), Cell::Empty, mph("79")],
            [Cell::Label(0xAB44_ED8B), Cell::Empty, DASHES],
            [Cell::Label(0x094B_FDFC), Cell::Empty, text("3")],
        ]
    );
}

#[test]
fn the_statistics_follow_the_racer_shown_and_the_unit() {
    let report = RaceReport::sample();
    let table = table(&report, Page::Statistics, 1, SpeedUnits::Metric);
    assert_eq!(table.rows[0][2], Cell::WithUnit("198".into(), 0x8569_A25F));
    assert_eq!(table.rows[2][2], text("2.69"), "the gap to the winner");
}

#[test]
fn the_stages_of_a_sprint_are_the_five_shares() {
    let report = RaceReport::sample();
    let table = table(&report, Page::Stages, 0, SpeedUnits::Imperial);
    assert_eq!(table.title, 0x34BA_50FF);
    assert_eq!(table.headers, [Some(0xE8B7_D527), Some(0x96B0_5F47), Some(0xB67D_A102)]);
    let rows: Vec<[Cell; 3]> = ["20%", "40%", "60%", "80%", "100%"]
        .iter()
        .zip(["30.36", "59.92", "1:35.19", "2:20.29", "2:52.51"])
        .map(|(share, time)| [text(share), text("1"), text(time)])
        .collect();
    assert_eq!(table.rows, rows);
    assert_eq!(table.footer, 0x8159_A0B2, "Standings");
}

#[test]
fn a_stage_the_racer_never_reached_is_dashes() {
    let mut report = RaceReport::sample();
    report.racers[3].stats.stages[2] = None;
    report.racers[3].stats.stages[3] = None;
    report.racers[3].time = None;
    let table = table(&report, Page::Stages, 3, SpeedUnits::Imperial);
    assert_eq!(table.rows[2], [text("60%"), DASHES, DASHES]);
    assert_eq!(table.rows[4], [text("100%"), DASHES, DASHES]);
}

#[test]
fn a_circuit_has_a_row_per_lap_and_a_lap_variance() {
    let report = RaceReport::sample_circuit();
    let stages = table(&report, Page::Stages, 0, SpeedUnits::Imperial);
    assert_eq!(stages.title, 0x9C8D_7FE8, "LAPS");
    assert_eq!(stages.headers[0], Some(0x0000_BF9C));
    assert_eq!(stages.rows.len(), 3);
    assert_eq!(stages.rows[1], [text("2"), text("2"), text("59.80")]);
    assert_eq!(stages.footer, 0x8159_A0B2);
    let stats = table(&report, Page::Statistics, 0, SpeedUnits::Imperial);
    assert_eq!(stats.footer, 0x7B8F_45DF, "Laps");
    assert_eq!(stats.rows.len(), 5);
    assert_eq!(stats.rows[3][0], Cell::Label(0x4121_E8C4));
    assert_eq!(stats.rows[3][2], text("1.40"));
}

#[test]
fn a_lap_not_run_is_dashes() {
    let mut report = RaceReport::sample_circuit();
    report.racers[2].laps.truncate(1);
    let stages = table(&report, Page::Stages, 2, SpeedUnits::Imperial);
    assert_eq!(stages.rows[2], [text("3"), DASHES, DASHES]);
    assert_eq!(lap_variance(&report.racers[2]), None, "one lap has no spread");
}

#[test]
fn a_drag_race_reads_its_marks_in_the_unit_of_the_player() {
    let mut report = RaceReport::sample();
    report.kind = RaceType::Drag;
    report.racers[0].stats.sixty_at = Some(4.5);
    report.racers[0].stats.quarter_at = Some(12.25);
    let imperial = table(&report, Page::Statistics, 0, SpeedUnits::Imperial);
    assert_eq!(imperial.rows[0], [Cell::Label(0xCCBC_22B3), Cell::Empty, text("4.50")]);
    assert_eq!(imperial.rows[1], [Cell::Label(0x49FD_5DCB), Cell::Empty, text("12.25")]);
    assert_eq!(imperial.rows.len(), 5);
    let metric = table(&report, Page::Statistics, 0, SpeedUnits::Metric);
    assert_eq!(
        (metric.rows[0][0].clone(), metric.rows[1][0].clone()),
        (Cell::Label(0xB8CF_16FC), Cell::Label(0x1C6F_2A82))
    );
}

#[test]
fn a_tollbooth_race_lists_its_tollbooths() {
    let mut report = RaceReport::sample();
    report.kind = RaceType::Tollbooth;
    report.racers[0].checkpoints = vec![20.0, 41.5];
    let stats = table(&report, Page::Statistics, 0, SpeedUnits::Imperial);
    assert_eq!(stats.rows.len(), 3, "no time behind");
    assert_eq!(stats.footer, 0xAC23_368C);
    let stages = table(&report, Page::Stages, 0, SpeedUnits::Imperial);
    assert_eq!(stages.title, 0xD10A_8EA2);
    assert_eq!(stages.headers[1], None, "the middle column is hidden");
    assert_eq!(stages.rows[1], [text("2"), Cell::Empty, text("41.50")]);
    assert_eq!(stages.rows.len(), 3);
}

#[test]
fn a_speed_trap_race_ranks_by_the_speeds() {
    let mut report = RaceReport::sample();
    report.kind = RaceType::SpeedTrap;
    report.racers[0].traps = vec![Some(40.0), Some(50.0)];
    report.racers[1].traps = vec![Some(45.0), None];
    let standings = table(&report, Page::Standings, 0, SpeedUnits::Imperial);
    assert_eq!(standings.headers[2], Some(0x7540_FB04));
    assert_eq!(standings.rows[0][2], Cell::WithUnit("201".into(), 0x8569_AB44));
    let stages = table(&report, Page::Stages, 0, SpeedUnits::Imperial);
    assert_eq!(stages.rows[0][1], text("2"), "a racer was faster at the first trap");
    assert_eq!(stages.rows[1][1], text("1"));
    let stats = table(&report, Page::Statistics, 1, SpeedUnits::Imperial);
    assert_eq!(stats.rows[1][2], DASHES, "one trap has no spread");
}

#[test]
fn a_racer_filter_with_no_racer_leaves_an_empty_page() {
    let report = RaceReport { racers: Vec::new(), ..RaceReport::sample() };
    assert!(table(&report, Page::Statistics, 0, SpeedUnits::Imperial).rows.is_empty());
    assert!(table(&report, Page::Stages, 0, SpeedUnits::Imperial).rows.is_empty());
}
