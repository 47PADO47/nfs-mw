//! The rap sheet pages behind the main page (`docs/specs/career-rapsheet.md` sections 4 to 8 and
//! `docs/specs/career-rapsheet-2.md` section 1): the summary, the infractions, the vehicle database, the cost to state,
//! the top 5 pursuits and their detail, and the rankings with their detail. The rules are in `nfsmw_career::rapsheet`;
//! each page reads the career, fills its objects and answers its keys. Values that pass between pages live in the
//! screen memory (see `common`).

pub(super) mod common;
mod cost;
mod infractions;
mod pursuit_detail;
mod ranking_detail;
mod rankings;
mod summary;
mod top_pursuits;
mod vehicles;

use crate::frontend::ids::screen;
use crate::frontend::logic::{Args, ScreenLogic};

/// Whether `name` (lower case) is the file `file`.
fn is(name: &str, file: &str) -> bool {
    name == file.to_ascii_lowercase()
}

/// The logic of the rap sheet page `name` (lower case), or `None` when `name` is not one of these pages.
pub fn make(name: &str, _args: Args) -> Option<Box<dyn ScreenLogic>> {
    if is(name, super::SUMMARY) {
        return Some(Box::new(summary::SummaryPage::new()));
    }
    if is(name, screen::RAP_SHEET_US) {
        return Some(Box::new(infractions::InfractionsPage::new()));
    }
    if is(name, screen::RAP_SHEET_VD) {
        return Some(Box::new(vehicles::VehiclesPage::new()));
    }
    if is(name, screen::RAP_SHEET_CTS) {
        return Some(Box::new(cost::CostPage::new()));
    }
    if is(name, super::TOP_PURSUITS) {
        return Some(Box::new(top_pursuits::TopPursuitsPage::new()));
    }
    if is(name, super::PURSUIT_DETAIL) {
        return Some(Box::new(pursuit_detail::PursuitDetailPage::new()));
    }
    if is(name, screen::RAP_SHEET_RANKINGS) {
        return Some(Box::new(rankings::RankingsPage::new()));
    }
    if is(name, screen::RAP_SHEET_RANKINGS_DETAIL) {
        return Some(Box::new(ranking_detail::RankingDetailPage::new()));
    }
    None
}
