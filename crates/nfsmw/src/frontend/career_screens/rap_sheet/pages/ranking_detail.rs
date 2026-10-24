//! The ranking detail (`RapSheetRankingsDetail.fng`): the thresholds of one statistic with the player's place among them,
//! ten rows at a time, the list scrolled so the player's row is in view (`docs/specs/career-rapsheet-2.md` section 1.4).
//! The statistic and the view come from the rankings page through the screen memory; Back returns to the rankings.

use blackbox_feng::ids::{PAD_BACK, PAD_DOWN, PAD_UP};
use nfsmw_career::rapsheet::rank::{
    DETAIL_TITLE_LABELS, TICKS_PER_SECOND, UNRANKED, initial_scroll, ranking_rows, rival_order,
};
use nfsmw_career::rapsheet::time::FLAGS_RANKING;
use nfsmw_career::rapsheet::{Page, PursuitHistory, RankingRow, Step, format_ticks};
use nfsmw_career::stable::Stable;

use nfsmw_data::rank_thresholds::RankThresholds;

use super::common::{
    Leave, RANKING_STAT, SWITCH_VIEW, back_to, car_name, header, load_views, number, put_label, put_text, remembered,
    store_views,
};
use super::rankings::{rank_of, stat_value, thresholds};
use crate::career::{Career, CareerHandle};
use crate::frontend::ids::{EXIT_COMPLETE, SCRIPT_HIGHLIGHT, SCRIPT_UNHIGHLIGHT};
use crate::frontend::logic::{Cx, ScreenLogic, wheel_as_rows};

/// The title label and the header name.
const TITLE: u32 = 0x8224_E17C;
const NAME: u32 = 0xEB40_6FEC;
/// The cells of the visible rows: rank, player or rival name, vehicle and time, each the first hash + row - 1. The
/// rank cell's hash is the category page's first rank cell (taken to be the same name in this package, unverified).
const RANK: u32 = 0x7711_109B;
const PLAYER_NAME: u32 = 0xBD1C_275C;
const VEHICLE: u32 = 0x2112_BA8F;
const TIME: u32 = 0x7C4B_7FFE;

/// Rows on the page at a time.
pub const VISIBLE: usize = 10;

/// The text of a threshold or the player's value: pursuit length as a time, the other statistics as numbers.
pub fn value_text(stat: usize, raw: u32) -> String {
    match stat == 0 {
        true => format_ticks(raw, FLAGS_RANKING),
        false => number(u64::from(raw)),
    }
}

/// The text of a threshold of the table. Pursuit length thresholds are seconds and show as ticks.
pub fn threshold_text(stat: usize, threshold: f32) -> String {
    match stat == 0 {
        true => format_ticks((threshold * TICKS_PER_SECOND + 0.5).trunc() as u32, FLAGS_RANKING),
        false => number(threshold.round() as u64),
    }
}

/// The rows of the page for a statistic and view.
pub fn rows(
    ranks: Option<&RankThresholds>,
    stable: &Stable,
    history: &PursuitHistory,
    stat: usize,
    lifetime: bool,
) -> Vec<RankingRow> {
    let Some(table) = thresholds(ranks, stat, lifetime) else { return Vec::new() };
    let player = rank_of(ranks, stable, history, stat, lifetime);
    ranking_rows(player, &table, &rival_order(stat, lifetime))
}

/// The window's top row so that the player's row (the row at `cursor`) is in view at the start.
pub fn start_top(cursor: usize, len: usize) -> usize {
    let rank = (cursor + 1).min(usize::from(u8::MAX)) as u8;
    usize::from(initial_scroll(rank)).min(len.saturating_sub(VISIBLE))
}

/// The top row after the cursor moved: the window follows the cursor.
pub fn follow(top: usize, cursor: usize, size: usize) -> usize {
    match (cursor < top, cursor >= top + size) {
        (true, _) => cursor,
        (_, true) => cursor + 1 - size,
        _ => top,
    }
}

/// The cursor after a step, staying on the rows.
pub fn move_cursor(cursor: usize, len: usize, step: Step) -> usize {
    match step {
        Step::Up => cursor.saturating_sub(1),
        Step::Down => (cursor + 1).min(len.saturating_sub(1)),
    }
}

/// The position of the player's row, 0 to 15 (its rank minus one).
pub fn player_index(rows: &[RankingRow]) -> usize {
    rows.iter().position(|row| matches!(row, RankingRow::Player { .. })).unwrap_or(0)
}

/// What the page shows: the statistic, the view and the rows with what the player's row names.
pub struct Board {
    pub stat: usize,
    pub lifetime: bool,
    pub rows: Vec<RankingRow>,
    pub name: String,
    pub value: u32,
    /// The car of the player's best pursuit (single pursuit view only).
    pub car: String,
}

fn build(career: &Career, stat: usize, lifetime: bool) -> Board {
    let ranks = career.data.ranks.as_ref();
    let car = match (lifetime, career.history.best.get(stat)) {
        (false, Some(best)) => car_name(career, best.car_key),
        _ => String::new(),
    };
    Board {
        stat,
        lifetime,
        rows: rows(ranks, &career.stable, &career.history, stat, lifetime),
        name: career.alias.clone(),
        value: stat_value(&career.stable, &career.history, stat, lifetime),
        car,
    }
}

/// The ranking detail page.
pub struct RankingDetailPage {
    board: Board,
    cursor: usize,
    top: usize,
    leave: Leave,
}

impl RankingDetailPage {
    pub fn new() -> Self {
        Self {
            board: Board {
                stat: 0,
                lifetime: false,
                rows: Vec::new(),
                name: String::new(),
                value: 0,
                car: String::new(),
            },
            cursor: 0,
            top: 0,
            leave: Leave::default(),
        }
    }

    /// Builds the board for the current statistic and view, and puts the cursor on the player's row.
    fn load(&mut self, cx: &mut Cx) {
        let (stat, lifetime) = (self.board.stat, self.board.lifetime);
        CareerHandle::shared().with(|career| {
            header(cx, career, NAME);
            self.board = build(career, stat, lifetime);
        });
        self.cursor = player_index(&self.board.rows);
        self.top = start_top(self.cursor, self.board.rows.len());
    }

    fn paint(&self, cx: &mut Cx) {
        put_label(cx, TITLE, DETAIL_TITLE_LABELS.get(self.board.stat).copied().unwrap_or(""));
        for slot in 0..VISIBLE {
            let n = slot as u32;
            match self.board.rows.get(self.top + slot) {
                Some(row) => self.paint_row(cx, n, row),
                None => clear_row(cx, n),
            }
            let script = match self.top + slot == self.cursor {
                true => SCRIPT_HIGHLIGHT,
                false => SCRIPT_UNHIGHLIGHT,
            };
            highlight(cx, slot + 1, script);
        }
    }

    fn paint_row(&self, cx: &mut Cx, n: u32, row: &RankingRow) {
        let board = &self.board;
        match row {
            RankingRow::Player { rank } => {
                paint_rank(cx, RANK + n, *rank);
                put_text(cx, PLAYER_NAME + n, &board.name);
                put_text(cx, TIME + n, &value_text(board.stat, board.value));
                match board.lifetime {
                    true => put_text(cx, VEHICLE + n, ""),
                    false => put_text(cx, VEHICLE + n, &board.car),
                }
            }
            RankingRow::Rival { rank, rival, threshold } => {
                put_text(cx, RANK + n, &rank.to_string());
                put_label(cx, PLAYER_NAME + n, &format!("BLACKLIST_RIVAL_{rival:02}_AKA"));
                put_text(cx, TIME + n, &threshold_text(board.stat, *threshold));
                match board.lifetime {
                    true => put_text(cx, VEHICLE + n, ""),
                    false => put_label(cx, VEHICLE + n, &format!("BLACKLIST_RIVAL_{rival:02}_CAR")),
                }
            }
        }
    }

    fn move_cursor(&mut self, cx: &mut Cx, step: Step) {
        self.cursor = move_cursor(self.cursor, self.board.rows.len(), step);
        self.top = follow(self.top, self.cursor, VISIBLE);
        self.paint(cx);
    }

    /// The switch key toggles this page's own view, then the rows are built again.
    fn toggle(&mut self, cx: &mut Cx) {
        let mut views = load_views(cx);
        views.toggle_detail();
        store_views(cx, views);
        self.board.lifetime = views.detail_lifetime;
        self.load(cx);
        self.paint(cx);
    }

    fn leave_to_rankings(&mut self, cx: &mut Cx) {
        let mut views = load_views(cx);
        views.close_detail();
        store_views(cx, views);
        self.leave.start(cx, back_to(Page::RankingDetail(self.board.stat as u8)));
    }
}

/// A rank cell: the number, or "--" when the player is unranked.
fn paint_rank(cx: &mut Cx, slot: u32, rank: u8) {
    match rank == UNRANKED {
        true => put_label(cx, slot, "RAP_RANKINGS_UNRANKED_RANK"),
        false => put_text(cx, slot, &rank.to_string()),
    }
}

fn clear_row(cx: &mut Cx, n: u32) {
    for base in [RANK, PLAYER_NAME, VEHICLE, TIME] {
        put_text(cx, base + n, "");
    }
}

/// Plays the highlight or the unhighlight script of the window's row `k` (1 to 10), named `POS<k>`.
fn highlight(cx: &mut Cx, k: usize, script: u32) {
    let Some(o) = cx.named(&format!("POS{k}")) else { return };
    cx.rt.run_script(o, script);
}

impl ScreenLogic for RankingDetailPage {
    fn start(&mut self, cx: &mut Cx) {
        self.board.stat = remembered(cx, RANKING_STAT).unwrap_or(0).min(9);
        self.board.lifetime = load_views(cx).detail_lifetime;
        self.load(cx);
        self.paint(cx);
    }

    fn scroll(&mut self, cx: &mut Cx, steps: i32) {
        wheel_as_rows(self, cx, steps);
    }

    fn message(&mut self, cx: &mut Cx, message: u32) {
        match message {
            PAD_UP => self.move_cursor(cx, Step::Up),
            PAD_DOWN => self.move_cursor(cx, Step::Down),
            SWITCH_VIEW => self.toggle(cx),
            PAD_BACK => self.leave_to_rankings(cx),
            EXIT_COMPLETE => self.leave.finish(cx),
            _ => {}
        }
    }
}

#[cfg(test)]
#[path = "ranking_detail_tests.rs"]
mod tests;
