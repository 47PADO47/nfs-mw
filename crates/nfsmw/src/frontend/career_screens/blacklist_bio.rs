//! The rival bio shown during the Blacklist sequence and from the reputation overview.

use super::{
    Args, Cx, DEFAULT_ASPECT, FIRST_RIVAL_STAGE, ListScreen, MenuList, MenuStyle, PAD_ACCEPT, PAD_BACK, Pick,
    RIVAL_OPENED_CAREER, RIVAL_OPENED_PLAIN, RivalStatus, ScreenLogic, UiAssets, career_rows, current_bin, game_text,
    go_home, label_hash, media, post_opening, rank_title, rival_entry, rival_key, start_bin, status_of,
};
use crate::frontend::career_screens::{RIVAL_BIO_CONTENT, reputation_overview};
use crate::frontend::flow::flow_first_visit::{bio_continue, enter_first_bin_on_open};

/// Objects of `SafehouseRivalBio.fng` by name hash.
const BIO_TITLE: u32 = 0x2426_57CE;
const BIO_NAME: u32 = 0x7ac3_d0c9;
const BIO_AKA: u32 = 0xb1f2_748d;
const BIO_CAR: u32 = 0xcb5b_f41a;
const BIO_STRENGTH: u32 = 0xa6f0_7bf3;
const BIO_TEXT: u32 = 0x27e1_d6d8;
const BIO_KEYBOARD_CONTINUE: u32 = 0xDDE3_DB34;

/// The rival bio (`SafeHouseRivalBio.fng`): the rival of the current bin.
pub(super) struct BioLogic {
    bin: u8,
    /// The rival the screen was opened for, or 0 for the career's current one.
    asked: u8,
    /// Whether a list look draws the page (`Cx::screen_list_look`, `ListScreen::Blacklist`).
    vertical: bool,
    /// Opened in a Blacklist sequence (`Args::sequence`).
    sequence: bool,
    /// The first visit: this bio entered bin 15, and leaving it goes on with the sequence (`flow_first_visit.rs`).
    first_visit: bool,
    /// Opened from the reputation overview (`Args::overview`): leaving goes back to it.
    overview: bool,
    /// What the screen was opened with, for the menu it goes back to.
    args: Args,
}

impl BioLogic {
    pub(super) fn new(args: Args) -> Self {
        Self {
            bin: FIRST_RIVAL_STAGE,
            asked: args.rival,
            vertical: false,
            sequence: args.sequence,
            first_visit: false,
            overview: args.overview,
            args,
        }
    }

    fn fill(&self, cx: &mut Cx) {
        if let Some(title) = rank_title(self.bin) {
            cx.text_group(BIO_TITLE, &title);
        }
        cx.label_group(BIO_NAME, label_hash(&rival_key(self.bin, "NAME")));
        cx.label_group(BIO_AKA, label_hash(&rival_key(self.bin, "AKA")));
        cx.label_group(BIO_CAR, label_hash(&rival_key(self.bin, "CAR")));
        cx.label_group(BIO_STRENGTH, label_hash(&rival_key(self.bin, "STRENGTH")));
        cx.label_group(BIO_TEXT, label_hash(&rival_key(self.bin, "BIO")));
        media::bind(cx, self.bin);
    }
}

impl ScreenLogic for BioLogic {
    fn footer(&self) -> crate::frontend::logic::Footer {
        if self.sequence && !self.vertical {
            crate::frontend::logic::Footer::RivalBioContinue
        } else {
            crate::frontend::logic::Footer::Standard
        }
    }

    fn start(&mut self, cx: &mut Cx) {
        self.vertical = cx.screen_list_look(ListScreen::Blacklist);
        // One open message by the context (`career-safehouse-2.md` 11.1 and 11.6): the Blacklist sequence's hides the Back
        // prompt and the side groups, the plain one shows them. The bio panel shows on its own message.
        let opened = if self.sequence { RIVAL_OPENED_CAREER } else { RIVAL_OPENED_PLAIN };
        post_opening(cx, &[opened]);
        if self.sequence {
            cx.rt.set_input_enabled(cx.package, false);
            // The PC footer root carries the start-hidden flag. The parent still controls when it appears.
            cx.hide(BIO_KEYBOARD_CONTINUE, false);
            self.first_visit = enter_first_bin_on_open();
        }
        if !self.sequence {
            post_opening(cx, &[RIVAL_BIO_CONTENT]);
        }
        if let Some(bin) = current_bin() {
            self.bin = start_bin(bin);
        }
        if self.asked != 0 {
            self.bin = start_bin(self.asked);
        }
        self.fill(cx);
    }

    fn tick(&mut self, cx: &mut Cx, _dt: f32) {
        self.vertical = cx.screen_list_look(ListScreen::Blacklist);
    }

    fn vertical(&self) -> bool {
        self.vertical
    }

    fn list(&self, _assets: &UiAssets) -> Option<MenuList> {
        if !self.vertical {
            return None;
        }
        let row = career_rows().into_iter().find(|row| row.bin == self.bin);
        let current = start_bin(current_bin().unwrap_or(FIRST_RIVAL_STAGE));
        let status = row.as_ref().map_or(RivalStatus::Locked, |row| status_of(row, current));
        let bounty = row.map_or(0, |row| row.bounty_needed);
        let template = game_text("TITLE_BLACKLIST_NUMBER");
        let title = blackbox_text::printf::fill(&template, &[self.bin.to_string().as_str()]);
        Some(MenuList {
            title,
            message: game_text(&rival_key(self.bin, "BIO")),
            entries: vec![rival_entry(self.bin, status, bounty)],
            selected: 0,
            first: 0,
            hints: Vec::new(),
            style: MenuStyle::Rival,
            nudge: None,
            scroll: 0.0,
            aspect: DEFAULT_ASPECT,
        })
    }

    fn pick_row(&mut self, _cx: &mut Cx, _row: usize, _pick: Pick) {}

    fn message(&mut self, cx: &mut Cx, message: u32) {
        match message {
            PAD_ACCEPT if self.first_visit => cx.send(bio_continue()),
            PAD_BACK if self.sequence => {}
            PAD_ACCEPT | PAD_BACK if self.overview => reputation_overview::open(cx, self.bin, self.args),
            PAD_ACCEPT | PAD_BACK => go_home(cx, self.args),
            _ => {}
        }
    }
}
