//! Session state of the rap sheet pages: the remembered selection, the ranking views and the end-game chain
//! (`docs/specs/career-rapsheet.md` section 1.3, `docs/specs/career-rapsheet-2.md` sections 1.1 and 3).

use crate::safehouse::FrontEndContext;

/// An entry of the remembered selection table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct MemoryEntry {
    package: u32,
    index: u8,
    context: u32,
}

/// The session-only table of remembered highlighted buttons: 50 entries of `{package name hash, byte index,
/// front-end context word}`. A lookup matches the same package whose context is `0xFFFFFFFF` or equals the current
/// context word; a set updates the matching entry or takes the first free one.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SelectionMemory {
    entries: Vec<MemoryEntry>,
}

/// Entries in the table.
pub const MEMORY_CAPACITY: usize = 50;
/// The context value that matches any context.
pub const ANY_CONTEXT: u32 = 0xFFFF_FFFF;

impl SelectionMemory {
    /// An empty table.
    pub fn new() -> SelectionMemory {
        SelectionMemory::default()
    }

    /// The remembered byte of a package for the context, if any.
    pub fn get(&self, package: u32, context: u32) -> Option<u8> {
        self.entries
            .iter()
            .find(|e| e.package == package && (e.context == ANY_CONTEXT || e.context == context))
            .map(|e| e.index)
    }

    /// Remembers a byte. Returns false when the table is full and nothing matched (the entry is dropped).
    pub fn set(&mut self, package: u32, context: u32, index: u8) -> bool {
        if let Some(e) =
            self.entries.iter_mut().find(|e| e.package == package && (e.context == ANY_CONTEXT || e.context == context))
        {
            e.index = index;
            return true;
        }
        if self.entries.len() >= MEMORY_CAPACITY {
            return false;
        }
        self.entries.push(MemoryEntry { package, index, context });
        true
    }
}

/// The button a page highlights on `INIT_COMPLETE`: the remembered byte, with 0 read as 1.
pub const fn highlight_button(byte: u8) -> u8 {
    match byte {
        0 => 1,
        b => b,
    }
}

/// The remembered numbers of the list pages (the values behind [`SelectionMemory`] for Main, TEP and Rankings).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Remembered {
    /// Main: the button 1 to 6 of the page last opened (default 1).
    pub main: u8,
    /// TEP: the entry 1 to 5 last opened (default 1).
    pub top_entry: u8,
    /// Rankings: the statistic index 0 to 9 last opened (default 0).
    pub ranking_stat: u8,
}

impl Default for Remembered {
    fn default() -> Self {
        Remembered { main: 1, top_entry: 1, ranking_stat: 0 }
    }
}

impl Remembered {
    /// Leaving Main with button `n` writes `n`.
    pub fn main_opened(&mut self, button: u8) {
        self.main = button;
    }

    /// Back from Main writes 1.
    pub fn main_back(&mut self) {
        self.main = 1;
    }

    /// Opening a pursuit detail from entry `n` writes `n`; Back from TEP resets it to 1.
    pub fn top_opened(&mut self, entry: u8) {
        self.top_entry = entry;
    }

    /// Back from TEP.
    pub fn top_back(&mut self) {
        self.top_entry = 1;
    }

    /// Opening a ranking detail writes the statistic index; Back from Rankings resets it to 0.
    pub fn ranking_opened(&mut self, stat: u8) {
        self.ranking_stat = stat;
    }

    /// Back from Rankings.
    pub fn ranking_back(&mut self) {
        self.ranking_stat = 0;
    }
}

/// The two view bytes of the rankings: the category page's and the detail's own, both starting at single pursuit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RankingViews {
    /// The category page view: `false` single pursuit (the start), `true` lifetime.
    pub page_lifetime: bool,
    /// The detail page view.
    pub detail_lifetime: bool,
}

impl RankingViews {
    /// The toggle key on the category page.
    pub fn toggle_page(&mut self) {
        self.page_lifetime = !self.page_lifetime;
    }

    /// The toggle key on the detail page.
    pub fn toggle_detail(&mut self) {
        self.detail_lifetime = !self.detail_lifetime;
    }

    /// Accept on a category copies the page view to the detail view.
    pub fn open_detail(&mut self) {
        self.detail_lifetime = self.page_lifetime;
    }

    /// Leaving the detail copies its view back to the page.
    pub fn close_detail(&mut self) {
        self.page_lifetime = self.detail_lifetime;
    }
}

/// The screens of the end-game chain, in order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EndGamePage {
    /// `Credits.fng`.
    Credits,
    /// `RapSheetLogin_ENDGAME.fng`.
    LoginEndgame,
    /// `RapSheetLogin2_ENDGAME.fng`.
    Login2Endgame,
    /// `RapSheetMain_ENDGAME.fng`.
    MainEndgame,
    /// `MainMenu_Sub.fng`.
    MainMenuSub,
}

/// The step counter of the end-game chain: winning the final event sets context bit 18,
/// resets the counter to 0 and requests `GameWon.fng`; every `EXIT_COMPLETE` of the chain runs [`EndGame::step`].
/// Where bit 18 is cleared again was not found.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct EndGame {
    /// The counter.
    pub counter: u32,
}

impl EndGame {
    /// The final event is won: context bit 18 is set and the counter resets.
    pub fn start(context: &mut FrontEndContext) -> EndGame {
        context.insert(FrontEndContext::GAME_WON);
        EndGame { counter: 0 }
    }

    /// The step routine: the page for the counter before the increment (0 credits, 1 login, 2 login 2, 3 main,
    /// 4 the safe house menu), then the counter goes up. Past 4 nothing opens.
    pub fn step(&mut self) -> Option<EndGamePage> {
        let page = match self.counter {
            0 => Some(EndGamePage::Credits),
            1 => Some(EndGamePage::LoginEndgame),
            2 => Some(EndGamePage::Login2Endgame),
            3 => Some(EndGamePage::MainEndgame),
            4 => Some(EndGamePage::MainMenuSub),
            _ => None,
        };
        self.counter = self.counter.saturating_add(1);
        page
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_selection_memory_matches_package_and_context() {
        let mut m = SelectionMemory::new();
        assert_eq!(m.get(0xAA, 1), None);
        assert!(m.set(0xAA, 1, 3));
        assert_eq!(m.get(0xAA, 1), Some(3));
        assert_eq!(m.get(0xAA, 2), None, "another context");
        assert_eq!(m.get(0xBB, 1), None);
        assert!(m.set(0xAA, 1, 5));
        assert_eq!(m.get(0xAA, 1), Some(5), "a set updates the matching entry");
        assert!(m.set(0xCC, ANY_CONTEXT, 2));
        assert_eq!(m.get(0xCC, 77), Some(2), "0xFFFFFFFF matches any context");
        assert_eq!(m.entries.len(), 2);
    }

    #[test]
    fn the_selection_memory_has_fifty_entries() {
        let mut m = SelectionMemory::new();
        for p in 0..50 {
            assert!(m.set(p, 0, 1));
        }
        assert!(!m.set(50, 0, 1), "full");
        assert!(m.set(10, 0, 4), "an existing entry can still change");
        assert_eq!(m.get(10, 0), Some(4));
    }

    #[test]
    fn the_highlight_reads_zero_as_one() {
        assert_eq!(highlight_button(0), 1);
        assert_eq!(highlight_button(4), 4);
    }

    #[test]
    fn the_remembered_numbers_follow_the_table() {
        let mut r = Remembered::default();
        assert_eq!((r.main, r.top_entry, r.ranking_stat), (1, 1, 0));
        r.main_opened(4);
        r.top_opened(3);
        r.ranking_opened(7);
        assert_eq!((r.main, r.top_entry, r.ranking_stat), (4, 3, 7));
        r.main_back();
        r.top_back();
        r.ranking_back();
        assert_eq!(r, Remembered::default());
    }

    #[test]
    fn the_ranking_views_copy_between_page_and_detail() {
        let mut v = RankingViews::default();
        assert!(!v.page_lifetime && !v.detail_lifetime, "single pursuit first");
        v.toggle_page();
        v.open_detail();
        assert!(v.detail_lifetime);
        v.toggle_detail();
        v.close_detail();
        assert!(!v.page_lifetime);
    }

    #[test]
    fn the_end_game_chain_runs_five_pages_in_order() {
        let mut c = FrontEndContext::CAREER;
        let mut g = EndGame::start(&mut c);
        assert!(c.contains(FrontEndContext::GAME_WON));
        let pages: Vec<Option<EndGamePage>> = (0..6).map(|_| g.step()).collect();
        assert_eq!(
            pages,
            [
                Some(EndGamePage::Credits),
                Some(EndGamePage::LoginEndgame),
                Some(EndGamePage::Login2Endgame),
                Some(EndGamePage::MainEndgame),
                Some(EndGamePage::MainMenuSub),
                None
            ]
        );
        assert_eq!(g.counter, 6);
    }
}
