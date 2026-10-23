//! What a theme draws of the LAN screens: the menu page and the session HUD, as plain data the LAN front end
//! (`frontend/lan`) publishes once a frame and the themes' `LanMenu` and `LanHud` slots read. The data names no layout
//! and no colour, as [`MenuList`] does not: an Original and a Remastered component each dress it their own way.
//!
//! The page is a [`MenuList`] (the rows, the selection, the footer hints, the window of rows that shows) plus the lines the
//! list has no place for: the line under the title, the reason something failed, and the side column (the lobby's chat).

use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use super::MenuList;

/// One LAN page.
#[derive(Clone, Debug, PartialEq)]
pub struct LanPage {
    /// The title, the rows (a row's value is a [`MenuValue::Choice`](super::MenuValue::Choice) for a choice or a typed
    /// text), the selection, the first row that shows and the footer hints.
    pub list: MenuList,
    /// The line under the title: the status, the room's mode, a countdown.
    pub subtitle: String,
    /// Why something failed, in the warning colour.
    pub error: Option<String>,
    /// The side column's heading and lines (the lobby's chat), oldest first. Empty for a page with none.
    pub side_title: String,
    pub side: Vec<String>,
}

/// What a feed line is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LanLineKind {
    Chat,
    Notice,
}

/// A line of the session feed with its opacity, 0 to 1.
#[derive(Clone, Debug, PartialEq)]
pub struct LanLine {
    pub kind: LanLineKind,
    pub text: String,
    pub opacity: f32,
}

/// What the spectator sees besides the world.
#[derive(Clone, Debug, PartialEq)]
pub struct LanSpectator {
    /// `Following Ann`, or `Free camera`.
    pub following: String,
    /// The racers, one line each.
    pub board: Vec<String>,
}

/// The HUD pieces of a session whose world has the screen.
#[derive(Clone, Debug, PartialEq)]
pub struct LanHud {
    pub feed: Vec<LanLine>,
    pub vote: Option<String>,
    pub countdown: Option<String>,
    pub spectator: Option<LanSpectator>,
    /// The window's width over its height, which sets where the screen's edges lie in FEng units.
    pub aspect: f32,
}

impl Default for LanHud {
    fn default() -> Self {
        Self { feed: Vec::new(), vote: None, countdown: None, spectator: None, aspect: super::DEFAULT_ASPECT }
    }
}

#[derive(Debug, Default)]
struct Inner {
    page: Option<LanPage>,
    hud: Option<LanHud>,
}

/// The LAN screens of this frame. Clones share one cell: the LAN front end fills it, and the components of the `LanMenu`
/// and `LanHud` slots read it. A page and a HUD are never on screen together.
#[derive(Clone, Debug, Default)]
pub struct LanCell {
    inner: Arc<Mutex<Inner>>,
}

impl LanCell {
    /// Shows `page` and nothing else of the LAN.
    pub fn set_page(&self, page: LanPage) {
        let mut inner = self.lock();
        inner.page = Some(page);
        inner.hud = None;
    }

    /// Shows `hud` and nothing else of the LAN.
    pub fn set_hud(&self, hud: LanHud) {
        let mut inner = self.lock();
        inner.hud = Some(hud);
        inner.page = None;
    }

    /// Shows nothing of the LAN.
    pub fn clear(&self) {
        let mut inner = self.lock();
        inner.page = None;
        inner.hud = None;
    }

    /// The page on screen, if any.
    pub fn page(&self) -> Option<LanPage> {
        self.lock().page.clone()
    }

    /// The HUD on screen, if any.
    pub fn hud(&self) -> Option<LanHud> {
        self.lock().hud.clone()
    }

    /// Whether a LAN page covers the screen (the front end's own screens are not drawn meanwhile).
    pub fn has_page(&self) -> bool {
        self.lock().page.is_some()
    }

    fn lock(&self) -> MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::theme::MenuStyle;

    fn page() -> LanPage {
        LanPage {
            list: MenuList {
                title: "LAN".into(),
                message: String::new(),
                entries: Vec::new(),
                selected: 0,
                first: 0,
                hints: Vec::new(),
                style: MenuStyle::Sub,
                nudge: None,
                scroll: 0.0,
                aspect: crate::ui::theme::DEFAULT_ASPECT,
            },
            subtitle: String::new(),
            error: None,
            side_title: String::new(),
            side: Vec::new(),
        }
    }

    #[test]
    fn a_page_and_a_hud_never_share_the_screen() {
        let cell = LanCell::default();
        let shared = cell.clone();
        cell.set_page(page());
        assert!(shared.has_page() && shared.hud().is_none());
        cell.set_hud(LanHud::default());
        assert!(!shared.has_page() && shared.hud().is_some());
        cell.clear();
        assert!(shared.page().is_none() && shared.hud().is_none());
    }
}
