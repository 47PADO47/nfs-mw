//! The rankings page (`RapSheetRankings.fng`): ten categories with the player's rank in each, in the single pursuit or
//! the lifetime view (`docs/specs/career-rapsheet-2.md` sections 1.1 and 1.2). Accept opens the detail of the category
//! with the statistic index; the detail's view starts as this page's view.

use blackbox_feng::ids::{PAD_ACCEPT, PAD_BACK};
use nfsmw_career::rapsheet::rank::{CATEGORY_ROWS, THRESHOLDS, UNRANKED, default_thresholds, rank};
use nfsmw_career::rapsheet::{Page, PursuitHistory, RankingViews, statistic};
use nfsmw_career::stable::Stable;
use nfsmw_data::rank_thresholds::RankThresholds;

use super::common::{
    Leave, RANKING_STAT, SWITCH_VIEW, back_to, go, header, load_views, put_label, put_text, remember, store_views,
};
use crate::career::{Career, CareerHandle};
use crate::frontend::ids::EXIT_COMPLETE;
use crate::frontend::logic::{Cx, ScreenLogic};

/// The title of the view, and the header name.
const TITLE: u32 = 0x001E_4FDA;
const NAME: u32 = 0xEB40_6FEC;
/// The rank cells `RANK_n` and the buttons `BUTTON_n`. Rows 1 to 9 are consecutive hashes, row 10 has its own.
const RANK_FIRST: u32 = 0x7711_109B;
const RANK_TENTH: u32 = 0x5933_242B;
const BUTTON_FIRST: u32 = 0xCDA0_A66B;
const BUTTON_TENTH: u32 = 0x81B5_73FB;

/// The rows of the page.
const ROWS: u32 = 10;

/// The rank cell of row `n` (1 to 10).
pub fn rank_slot(n: u32) -> u32 {
    match n {
        10 => RANK_TENTH,
        _ => RANK_FIRST + n - 1,
    }
}

/// The button of row `n` (1 to 10).
pub fn button_slot(n: u32) -> u32 {
    match n {
        10 => BUTTON_TENTH,
        _ => BUTTON_FIRST + n - 1,
    }
}

/// The thresholds of a statistic in a view: the install's tables when the data read them, else the crate's defaults.
pub fn thresholds(ranks: Option<&RankThresholds>, stat: usize, lifetime: bool) -> Option<[f32; THRESHOLDS]> {
    match ranks {
        Some(ranks) => {
            let tables = match lifetime {
                true => &ranks.lifetime,
                false => &ranks.single,
            };
            tables.get(stat).copied()
        }
        None => default_thresholds(stat, lifetime).copied(),
    }
}

/// The value the rank rule compares: the career total (lifetime) or the best single pursuit.
pub fn stat_value(stable: &Stable, history: &PursuitHistory, stat: usize, lifetime: bool) -> u32 {
    match lifetime {
        true => statistic(stable, history, stat).unwrap_or(0),
        false => history.best.get(stat).map_or(0, |best| best.value),
    }
}

/// The player's rank in a statistic and view; unranked when the table is missing.
pub fn rank_of(
    ranks: Option<&RankThresholds>,
    stable: &Stable,
    history: &PursuitHistory,
    stat: usize,
    lifetime: bool,
) -> u8 {
    let Some(table) = thresholds(ranks, stat, lifetime) else { return UNRANKED };
    rank(stat, stat_value(stable, history, stat, lifetime), &table)
}

/// The player's rank in each of the ten rows, in page order (row `n` is the statistic `CATEGORY_ROWS[n - 1]`).
pub fn categories(
    ranks: Option<&RankThresholds>,
    stable: &Stable,
    history: &PursuitHistory,
    lifetime: bool,
) -> Vec<u8> {
    CATEGORY_ROWS.iter().map(|&stat| rank_of(ranks, stable, history, stat, lifetime)).collect()
}

/// The title label of a view.
pub fn view_title(lifetime: bool) -> &'static str {
    match lifetime {
        true => "RAP_RANKINGS_CAREER_PURSUIT_VIEW",
        false => "RAP_RANKINGS_SINGLE_PURSUIT_VIEW",
    }
}

/// The rankings category page.
pub struct RankingsPage {
    leave: Leave,
}

impl RankingsPage {
    pub fn new() -> Self {
        Self { leave: Leave::default() }
    }

    fn refresh(&self, cx: &mut Cx) {
        CareerHandle::shared().with(|career| self.render(cx, career));
    }

    fn render(&self, cx: &mut Cx, career: &Career) {
        let lifetime = load_views(cx).page_lifetime;
        header(cx, career, NAME);
        put_label(cx, TITLE, view_title(lifetime));
        let ranks = categories(career.data.ranks.as_ref(), &career.stable, &career.history, lifetime);
        for (slot, rank) in ranks.into_iter().enumerate() {
            paint_rank(cx, rank_slot(slot as u32 + 1), rank);
        }
    }

    /// The row whose button has the focus, 0 to 9.
    fn focused_row(cx: &Cx) -> Option<usize> {
        let focus = cx.rt.current_button(cx.package)?;
        (1..=ROWS).find(|&n| cx.object(button_slot(n)) == Some(focus)).map(|n| n as usize - 1)
    }

    fn open_detail(&mut self, cx: &mut Cx) {
        let Some(row) = Self::focused_row(cx) else { return };
        let stat = CATEGORY_ROWS[row];
        let mut views: RankingViews = load_views(cx);
        views.open_detail();
        store_views(cx, views);
        remember(cx, RANKING_STAT, stat);
        self.leave.start(cx, go(Page::RankingDetail(stat as u8)));
    }

    fn toggle(&self, cx: &mut Cx) {
        let mut views = load_views(cx);
        views.toggle_page();
        store_views(cx, views);
        self.refresh(cx);
    }
}

/// A rank cell: the number, or the word "UNRANKED".
fn paint_rank(cx: &mut Cx, slot: u32, rank: u8) {
    match rank == UNRANKED {
        true => put_label(cx, slot, "RAP_RANKINGS_UNRANKED"),
        false => put_text(cx, slot, &rank.to_string()),
    }
}

impl ScreenLogic for RankingsPage {
    fn start(&mut self, cx: &mut Cx) {
        self.refresh(cx);
    }

    fn message(&mut self, cx: &mut Cx, message: u32) {
        match message {
            SWITCH_VIEW => self.toggle(cx),
            PAD_ACCEPT => self.open_detail(cx),
            PAD_BACK => {
                remember(cx, RANKING_STAT, 0);
                self.leave.start(cx, back_to(Page::Rankings));
            }
            EXIT_COMPLETE => self.leave.finish(cx),
            _ => {}
        }
    }
}

#[cfg(test)]
#[path = "rankings_tests.rs"]
mod tests;
