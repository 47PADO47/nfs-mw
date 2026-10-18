//! What the rap sheet pages share: the language text, the figures as the pages print them, the header objects, the car
//! names, the leave animation and the screen memory the pages pass on (`docs/specs/career-text.md` section 6,
//! `docs/specs/career-rapsheet.md` sections 1.3 and 1.4).

use blackbox_text::label_hash;
use nfsmw_career::carlot::by_frontend_key;
use nfsmw_career::rapsheet::{Page, RankingViews, back_target};

use super::super::main_page::serial_text;
use super::super::open;
use crate::career::Career;
use crate::frontend::ids::LEAVE_SCREEN;
use crate::frontend::infractions::money;
use crate::frontend::logic::{Command, Cx};
use crate::ui::Text;

/// The case file header object, `CASE FILE: %s` with the serial (the same object on every page).
pub const CASE_FILE: u32 = 0x1232_703A;
/// The switch key of the infractions and the rankings pages (`0xC519BFC4`, "Switch View").
pub const SWITCH_VIEW: u32 = 0xC519_BFC4;

/// The placeholders a language string uses for a value.
const TOKENS: [&str; 3] = ["%$d", "%$.0f", "%s"];

/// Screen memory keys. The top 5 entry (1 to 5) last opened is the pursuit detail's argument, kept under the
/// [`TOP_ENTRY`] key; the rankings keep the statistic the detail opens with and the two view flags.
pub const TOP_ENTRY: &str = "RapSheetTEP";
pub const RANKING_STAT: &str = "RapSheetRankings";
pub const RANKING_PAGE_VIEW: &str = "RapSheetRankings view";
pub const RANKING_DETAIL_VIEW: &str = "RapSheetRankingsDetail view";

/// The template filled with `value` at its first placeholder; a template without one is returned as it is.
pub fn fill(template: &str, value: &str) -> String {
    let found = TOKENS.iter().filter_map(|token| template.find(token).map(|at| (at, token.len()))).min();
    let Some((at, len)) = found else { return template.to_owned() };
    format!("{}{value}{}", &template[..at], &template[at + len..])
}

/// Bytes of a zero-terminated text (a case id, a serial) up to the first zero.
pub fn text_to_nul(bytes: &[u8]) -> String {
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).into_owned()
}

/// A figure as the pages print it: digits with a comma every three places, saturating at 32 bits like the ticket.
pub fn number(value: u64) -> String {
    money(u32::try_from(value).unwrap_or(u32::MAX))
}

/// Sets an object's text.
pub fn put_text(cx: &mut Cx, name: u32, text: &str) {
    let Some(o) = cx.object(name) else { return };
    cx.rt.set_text(o, text);
}

/// Shows a language label on an object.
pub fn put_label(cx: &mut Cx, name: u32, key: &str) {
    let Some(o) = cx.object(name) else { return };
    cx.rt.set_label(o, label_hash(key));
}

/// Shows a language string with a value in it. Without the string in the table the value alone is shown.
pub fn put_value(cx: &mut Cx, name: u32, key: &str, value: &str) {
    let text = match Text::Game(label_hash(key)).resolve(cx.assets) {
        Some(template) => fill(&template, value),
        None => value.to_owned(),
    };
    put_text(cx, name, &text);
}

/// The case file and the player's name, the header of every page.
pub fn header(cx: &mut Cx, career: &Career, name: u32) {
    put_value(cx, CASE_FILE, "RAP_CASE_FILE", &serial_text(&career.settings.serial));
    put_value(cx, name, "RAP_PLAYER_NAME", &career.alias);
}

/// The name of a car type from its front-end key, from the car lot's catalogue. Empty when the catalogue has no such car.
pub fn car_name(career: &Career, frontend_key: u32) -> String {
    by_frontend_key(&career.data.cars, frontend_key).map(|car| car.name.clone()).unwrap_or_default()
}

/// The command that opens a rap sheet page from the safe house.
pub fn go(page: Page) -> Command {
    open(page)
}

/// The command for Back from `page`: the page the rap sheet graph returns to.
pub fn back_to(page: Page) -> Command {
    open(back_target(page))
}

/// A page leaving: Back posts the package's leave, and its `EXIT_COMPLETE` sends the command the leave was for.
#[derive(Default)]
pub struct Leave {
    to: Option<Command>,
}

impl Leave {
    /// Starts the leave, once; a second request during the animation is ignored.
    pub fn start(&mut self, cx: &mut Cx, to: Command) {
        if self.to.is_some() {
            return;
        }
        self.to = Some(to);
        cx.rt.post_to_package(cx.package, LEAVE_SCREEN);
    }

    /// The package's leave animation ended: the screen goes on.
    pub fn finish(&mut self, cx: &mut Cx) {
        let Some(to) = self.to.take() else { return };
        cx.send(to);
    }
}

/// A number kept in the screen memory, or `None` when the screen never left one.
pub fn remembered(cx: &Cx, key: &str) -> Option<usize> {
    cx.memory.get(key).copied()
}

/// Keeps a number in the screen memory for the next screen that asks for it.
pub fn remember(cx: &mut Cx, key: &str, value: usize) {
    cx.memory.insert(key.to_owned(), value);
}

/// The two ranking views of the session.
pub fn load_views(cx: &Cx) -> RankingViews {
    RankingViews { page_lifetime: flag(cx, RANKING_PAGE_VIEW), detail_lifetime: flag(cx, RANKING_DETAIL_VIEW) }
}

/// Keeps the two ranking views of the session.
pub fn store_views(cx: &mut Cx, views: RankingViews) {
    remember(cx, RANKING_PAGE_VIEW, usize::from(views.page_lifetime));
    remember(cx, RANKING_DETAIL_VIEW, usize::from(views.detail_lifetime));
}

fn flag(cx: &Cx, key: &str) -> bool {
    remembered(cx, key).is_some_and(|value| value != 0)
}

#[cfg(test)]
#[path = "common_tests.rs"]
mod tests;
