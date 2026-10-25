//! What the LAN front end hands the themes each frame: the page as a [`LanPage`] (the rows as a menu list, the footer hints
//! as keys) and the session HUD as a [`LanHud`], both plain data (`ui/theme/lan.rs`). The themes' `LanMenu` and `LanHud`
//! components draw them; nothing here names a layout or a colour. It also keeps what a click needs of the page that was drawn.

use super::feed::FeedKind;
use super::hud::HudView;
use super::menu::rows::{HintDoes, HintKey, PageView, Row, RowKind};
use crate::input::Action;
use crate::ui::prompts::Prompt;
use crate::ui::theme::custom::menu::hint_at as custom_hint_at;
use crate::ui::theme::custom::menu::layout::{self as custom_place, MenuFrame, Side as CustomSide};
use crate::ui::theme::remastered::menu::layout as place;
use crate::ui::theme::{
    LanHud, LanLine, LanLineKind, LanPage, LanSpectator, MenuEntry, MenuHint, MenuList, MenuStyle, MenuValue,
};

/// The page as the themes draw it. `first` is the row that shows first (the window of rows follows the selection) and
/// `aspect` the window's width over its height.
pub fn page_of(view: &PageView, first: usize, aspect: f32) -> LanPage {
    let list = MenuList {
        title: view.title.clone(),
        message: String::new(),
        entries: view.rows.iter().map(entry_of).collect(),
        selected: view.selected,
        first,
        hints: view.hints.iter().map(|&(key, does)| hint_of(key, does)).collect(),
        style: MenuStyle::Sub,
        nudge: None,
        scroll: 0.0,
        aspect,
    };
    LanPage {
        list,
        subtitle: view.subtitle.clone(),
        error: view.error.clone(),
        side_title: view.side_title.clone(),
        side: view.side.clone(),
    }
}

/// A row as a menu entry: an action has no value, a choice shows its value between chevrons, and a typed text or a line of
/// information shows its value without them.
fn entry_of(row: &Row) -> MenuEntry {
    let value = match (row.kind, row.value.is_empty()) {
        (RowKind::Action, _) | (_, true) => MenuValue::None,
        (RowKind::Choice, false) => MenuValue::Choice { text: row.value.clone(), arrows: row.enabled },
        (RowKind::Text { .. } | RowKind::Info, false) => MenuValue::Choice { text: row.value.clone(), arrows: false },
    };
    MenuEntry { label: row.label.clone(), enabled: row.enabled, value, icon: None }
}

fn hint_of(key: HintKey, does: HintDoes) -> MenuHint {
    MenuHint { keys: vec![Prompt::Key(key.cap())], label: does.label(), action: hint_action(does) }
}

/// The action a click on a hint presses: the hints that select (or continue) press accept, those that go back (or stop
/// typing) press back, and the ones that only say how to move do nothing.
fn hint_action(does: HintDoes) -> Option<Action> {
    match does {
        HintDoes::Back | HintDoes::StopTyping => Some(Action::MenuBack),
        HintDoes::Select | HintDoes::SelectOrType | HintDoes::SelectOrSay | HintDoes::Continue => {
            Some(Action::MenuAccept)
        }
        HintDoes::Change | HintDoes::Move => None,
    }
}

/// The HUD as the themes draw it.
pub fn hud_of(hud: &HudView, aspect: f32) -> LanHud {
    LanHud {
        feed: hud
            .feed
            .iter()
            .map(|(kind, text, opacity)| LanLine {
                kind: match kind {
                    FeedKind::Chat => LanLineKind::Chat,
                    FeedKind::Notice => LanLineKind::Notice,
                },
                text: text.clone(),
                opacity: *opacity,
            })
            .collect(),
        vote: hud.vote.clone(),
        countdown: hud.countdown.clone(),
        spectator: hud
            .spectator
            .as_ref()
            .map(|panel| LanSpectator { following: panel.following.clone(), board: panel.board.clone() }),
        aspect,
    }
}

/// Where the page drawn last put its rows and hints, for a click on them (the same geometry the themes draw by: the
/// Original and Remastered pages share the Remastered places, the Custom page has its own).
#[derive(Clone, Debug, Default)]
pub struct ClickMap {
    pub first: usize,
    pub count: usize,
    pub wide: bool,
    pub aspect: f32,
    pub hints: Vec<MenuHint>,
    /// Which rows are choices: a click on a chevron steps them.
    pub choices: Vec<bool>,
    /// The `[custom_hud]` text size when the Custom look draws the page (`custom/menu/lan.rs`), `None` for the
    /// Remastered places.
    pub custom: Option<f32>,
}

impl ClickMap {
    pub fn of(view: &PageView, page: &LanPage) -> Self {
        Self {
            first: page.list.first,
            count: page.list.entries.len(),
            wide: page.list.has_values(),
            aspect: page.list.aspect,
            hints: page.list.hints.clone(),
            choices: view.rows.iter().map(|row| row.kind == RowKind::Choice).collect(),
            custom: None,
        }
    }

    /// This map for a page the Custom look draws, at `[custom_hud] text_size`, or for the Remastered places (`None`).
    pub fn in_custom(self, text_size: Option<f32>) -> Self {
        Self { custom: text_size, ..self }
    }

    /// The row under `point` (FEng units), and which part of it.
    pub fn row_at(&self, point: glam::Vec2) -> Option<(usize, place::Side)> {
        let Some(text_size) = self.custom else {
            let shown = place::shown(self.count);
            let row = place::row_at(point, shown, self.aspect, self.wide)? + self.first;
            let side = place::side_at(point.x - place::left_edge(self.aspect));
            return Some((row, side));
        };
        let frame = MenuFrame::new(self.aspect, text_size);
        let shown = custom_place::shown(self.count.saturating_sub(self.first));
        let row = frame.row_at(point, shown, self.wide)? + self.first;
        let side = match frame.side_at(point.x) {
            CustomSide::Left => place::Side::Left,
            CustomSide::Middle => place::Side::Middle,
            CustomSide::Right => place::Side::Right,
        };
        Some((row, side))
    }

    /// The action of the footer hint under `point`.
    pub fn hint_at(&self, point: glam::Vec2) -> Option<Action> {
        match self.custom {
            Some(text_size) => custom_hint_at(&self.hints, point, self.aspect, text_size),
            None => place::hint_at(&self.hints, point, self.aspect),
        }
    }
}

/// The first row to show: the window stays where it is while the selection moves inside it (`place::window_start`).
pub fn window_first(first: usize, selected: usize, count: usize) -> usize {
    place::window_start(first, selected, count)
}

#[cfg(test)]
mod tests;
