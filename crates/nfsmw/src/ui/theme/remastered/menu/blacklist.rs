//! The Blacklist in the Remastered look: the title and the count of rivals beaten at the top, the fifteen rivals as rows (rank,
//! picture, name, bounty and where the player stands with them), the selected row on a slanted bar, and the selected rival's
//! card at the right of the screen. The geometry is shared with the frontend's hit test.

use blackbox_ui::{Image, Rect, Slant};
use glam::Vec2;

use super::layout as place;
use super::{BAR_STOPS, Parts, fitted};
use crate::ui::theme::remastered::skin::keys;
use crate::ui::theme::{MenuEntry, MenuList, MenuValue, RivalInfo, RivalStatus};

/// The rows' bars: where the bar starts from the screen's left edge, its width, and its height and lean.
const BAR_LEFT: f32 = 30.0;
const BAR_WIDTH: f32 = 440.0;
const BAR_HEIGHT: f32 = 21.0;
const LEAN: f32 = 6.0;
/// The first row's y, and the distance between rows. Fifteen rows end above the footer.
const FIRST_Y: f32 = -166.0;
const PITCH: f32 = 23.0;
const TEXT_SCALE: f32 = 1.5;
const RANK_SCALE: f32 = 1.55;
/// The columns, from the bar's left edge: the rank's right edge, the picture, the name, the bounty's and the status's right edge.
const RANK_RIGHT: f32 = 34.0;
const PICTURE_LEFT: f32 = 44.0;
const PICTURE: f32 = 19.0;
const NAME_LEFT: f32 = 74.0;
const NAME_LIMIT: f32 = 170.0;
const BOUNTY_RIGHT: f32 = 318.0;
const STATUS_RIGHT: f32 = BAR_WIDTH - 12.0;
/// The title's y, and how far the card is from the end of the bars, from the screen's right edge, and the least width it has.
const TITLE_Y: f32 = -208.0;
const CARD_GAP: f32 = 36.0;
const CARD_MARGIN: f32 = 44.0;
const CARD_LEAST: f32 = 120.0;
/// The picture of a rival not reached yet is dimmed.
const LOCKED_PICTURE: [u8; 4] = [70, 72, 80, 255];

/// The y of the middle of the `i`th row.
pub fn row_y(i: usize) -> f32 {
    FIRST_Y + PITCH * i as f32
}

/// The row of a list under a point of the screen.
pub fn row_at(point: Vec2, list: &MenuList) -> Option<usize> {
    let left = place::left_edge(list.aspect) + BAR_LEFT;
    if point.x < left || point.x > left + BAR_WIDTH {
        return None;
    }
    let row = ((point.y - (FIRST_Y - PITCH / 2.0)) / PITCH).floor();
    (row >= 0.0 && (row as usize) < list.entries.len()).then_some(row as usize)
}

/// The rectangle the selected rival's card takes at the right of the screen, or `None` when the window is too narrow for it.
pub(super) fn card_area(aspect: f32) -> Option<Rect> {
    let left = place::left_edge(aspect) + BAR_LEFT + BAR_WIDTH + CARD_GAP;
    let right = place::right_edge(aspect) - CARD_MARGIN;
    let width = right - left;
    (width >= CARD_LEAST).then(|| Rect::new(left + width / 2.0, FIRST_Y - PITCH / 2.0 + 90.0, width, 180.0))
}

/// The `i`th entry of a list with the rival it holds.
pub(super) fn rival_at(list: &MenuList, i: usize) -> Option<(&MenuEntry, &RivalInfo)> {
    let entry = list.entries.get(i)?;
    match &entry.value {
        MenuValue::Rival(info) => Some((entry, info)),
        _ => None,
    }
}

/// The colour of a status: the accent for a rival beaten, white for the one in hand, dim for the rest.
pub(super) fn status_colour(status: RivalStatus) -> &'static str {
    match status {
        RivalStatus::Complete => keys::MENU_TITLE,
        RivalStatus::InProgress => keys::MENU_TEXT_SELECTED,
        RivalStatus::Locked => keys::MENU_TEXT_DISABLED,
    }
}

/// A status as the screen writes it.
pub(super) fn status_text(status: RivalStatus) -> &'static str {
    match status {
        RivalStatus::Complete => "COMPLETE",
        RivalStatus::InProgress => "IN PROGRESS",
        RivalStatus::Locked => "LOCKED",
    }
}

impl Parts<'_, '_> {
    /// The Blacklist: a scrim, the shade of the left, the title and count, a row per rival, the selected rival's card and the
    /// footer.
    pub(super) fn blacklist(&mut self, list: &MenuList) {
        let left = place::left_edge(list.aspect);
        self.scrim(list.aspect);
        self.shade(left);
        let title = (left + place::TEXT_LEFT, TITLE_Y);
        self.text(
            keys::FONT_MENU_TITLE,
            &list.title.to_uppercase(),
            keys::MENU_TITLE,
            place::TITLE_SCALE,
            title,
            false,
        );
        let count = (left + BAR_LEFT + BAR_WIDTH, TITLE_Y);
        self.text(keys::FONT_MENU, &list.message, keys::MENU_TAGLINE, 1.6, count, true);
        for (i, entry) in list.entries.iter().enumerate() {
            self.rival_row(entry, i == list.selected, row_y(i), left + BAR_LEFT);
        }
        if let (Some(area), Some((entry, info))) = (card_area(list.aspect), rival_at(list, list.selected)) {
            self.rival_card(&entry.label, info, area);
        }
        self.footer(&list.hints, left);
    }

    /// One rival's row: rank, picture, name, bounty and status, on a bar when it is the selected one.
    fn rival_row(&mut self, entry: &MenuEntry, selected: bool, y: f32, bar_left: f32) {
        let MenuValue::Rival(info) = &entry.value else { return };
        let locked = info.status == RivalStatus::Locked;
        let colour = match (locked, selected) {
            (true, _) => keys::MENU_TEXT_DISABLED,
            (false, true) => keys::MENU_TEXT_SELECTED,
            (false, false) => keys::MENU_TEXT,
        };
        if selected {
            let rect = Rect::new(bar_left + BAR_WIDTH / 2.0, y, BAR_WIDTH, BAR_HEIGHT);
            self.add(Slant::at_default_depth(keys::MENU_BAR, rect, LEAN, &BAR_STOPS, self.cx));
        }
        self.text(keys::FONT_MENU, &info.rank.to_string(), colour, RANK_SCALE, (bar_left + RANK_RIGHT, y), true);
        if info.portrait != 0 {
            let tint = if locked { LOCKED_PICTURE } else { [255; 4] };
            let rect = Rect::new(bar_left + PICTURE_LEFT + PICTURE / 2.0, y, PICTURE, PICTURE);
            self.push(Image::raw(info.portrait).with_tint(tint), rect);
        }
        let name = entry.label.to_uppercase();
        let scale = fitted(&name, TEXT_SCALE, NAME_LIMIT);
        self.text(keys::FONT_MENU, &name, colour, scale, (bar_left + NAME_LEFT, y), false);
        self.text(
            keys::FONT_MENU,
            &format!("${}", info.bounty),
            colour,
            TEXT_SCALE,
            (bar_left + BOUNTY_RIGHT, y),
            true,
        );
        let status = (status_text(info.status), status_colour(info.status));
        self.text(keys::FONT_MENU, status.0, status.1, TEXT_SCALE, (bar_left + STATUS_RIGHT, y), true);
    }
}

#[cfg(test)]
mod tests {
    use blackbox_ui::{BuildCx, DrawList, LayoutCx, NodeKind, Theme, UiNode};

    use super::*;
    use crate::ui::theme::custom::{DriveFeedCell, palette};
    use crate::ui::theme::remastered::{RemasteredTheme, palette_skin, skin};
    use crate::ui::theme::{ElementId, MenuListCell, MenuStyle, RivalInfo};

    const ASPECT: f32 = 16.0 / 9.0;

    fn rival(rank: u8, status: RivalStatus) -> MenuEntry {
        let info = RivalInfo {
            rank,
            portrait: 1000 + u32::from(rank),
            alias: "Baron".into(),
            car: "Porsche".into(),
            strength: "Infractions".into(),
            bounty: format!("{rank},000"),
            status,
        };
        MenuEntry { label: format!("Rival {rank}"), enabled: true, value: MenuValue::Rival(info) }
    }

    fn blacklist(selected: usize, aspect: f32) -> MenuList {
        let status = |rank: u8| match rank {
            11..=15 => RivalStatus::Complete,
            10 => RivalStatus::InProgress,
            _ => RivalStatus::Locked,
        };
        MenuList {
            title: "Blacklist".into(),
            message: "5/15".into(),
            entries: (1..=15u8).rev().map(|rank| rival(rank, status(rank))).collect(),
            selected,
            first: 0,
            hints: Vec::new(),
            style: MenuStyle::Blacklist,
            nudge: None,
            scroll: 0.0,
            aspect,
        }
    }

    fn built(list: MenuList) -> Vec<UiNode> {
        let cell = MenuListCell::new();
        cell.set(ElementId::Blacklist.slot(), list);
        let skin = palette::remastered().skin().merged(skin()).merged(palette_skin(&palette::remastered()));
        let theme = RemasteredTheme::new(
            Default::default(),
            DriveFeedCell::new(),
            Default::default(),
            cell,
            Default::default(),
        );
        let mut menu = theme.element(ElementId::Blacklist.slot(), &BuildCx::new(&skin)).expect("Remastered draws it");
        menu.layout(&LayoutCx::new(&skin, 0.0), blackbox_ui::Rect::SCREEN);
        let mut out = DrawList::new();
        menu.build(&mut out);
        out.iter().cloned().collect()
    }

    fn texts(nodes: &[UiNode]) -> Vec<&str> {
        nodes.iter().filter_map(|node| node.text.as_deref()).collect()
    }

    fn pictures(nodes: &[UiNode], texture: u32) -> usize {
        nodes.iter().filter(|n| matches!(n.kind, NodeKind::Image { texture: t, .. } if t == texture)).count()
    }

    #[test]
    fn the_fifteen_rows_fit_above_the_footer_and_a_point_finds_its_row() {
        let list = blacklist(0, ASPECT);
        assert!(row_y(14) + BAR_HEIGHT / 2.0 < place::RULE_TOP_Y, "the last bar clears the footer's rule");
        let left = place::left_edge(ASPECT) + BAR_LEFT;
        assert_eq!(row_at(Vec2::new(left + 100.0, row_y(0)), &list), Some(0));
        assert_eq!(row_at(Vec2::new(left + 100.0, row_y(14) + PITCH * 0.49), &list), Some(14));
        assert_eq!(row_at(Vec2::new(left + 100.0, row_y(14) + PITCH), &list), None, "below the last rival");
        assert_eq!(row_at(Vec2::new(left + 100.0, row_y(0) - PITCH), &list), None, "above the first");
        assert_eq!(row_at(Vec2::new(left - 2.0, row_y(3)), &list), None, "left of the bars");
        assert_eq!(row_at(Vec2::new(left + BAR_WIDTH + 2.0, row_y(3)), &list), None, "right of the bars");
    }

    #[test]
    fn every_rival_has_a_row_with_its_name_bounty_status_and_picture() {
        let nodes = built(blacklist(5, ASPECT));
        let drawn = texts(&nodes);
        for rank in 1..=15u8 {
            assert!(drawn.contains(&format!("RIVAL {rank}").as_str()), "rival {rank}");
            assert!(drawn.contains(&format!("${rank},000").as_str()), "bounty {rank}");
        }
        assert_eq!(drawn.iter().filter(|text| **text == "COMPLETE").count(), 5);
        assert_eq!(drawn.iter().filter(|text| **text == "LOCKED").count(), 9);
        assert_eq!(drawn.iter().filter(|text| **text == "IN PROGRESS").count(), 2, "its row and the card");
        assert!(drawn.contains(&"BLACKLIST") && drawn.contains(&"5/15"));
        assert_eq!(pictures(&nodes, 1001), 1, "an unselected rival's picture is in its row only");
        assert_eq!(pictures(&nodes, 1010), 1 + 1, "the selected rival's picture is in its row and on the card");
    }

    #[test]
    fn the_card_follows_the_selection_and_gives_way_in_a_narrow_window() {
        let nodes = built(blacklist(0, ASPECT));
        assert!(texts(&nodes).contains(&"BOUNTY") && texts(&nodes).contains(&"BARON"));
        assert_eq!(texts(&nodes).iter().filter(|text| **text == "RIVAL 15").count(), 2, "row and card");
        let narrow = built(blacklist(0, 4.0 / 3.0));
        assert!(!texts(&narrow).contains(&"BOUNTY"), "no room for the card at 4:3");
        assert!(card_area(4.0 / 3.0).is_none() && card_area(ASPECT).is_some());
    }

    #[test]
    fn the_bio_page_shows_the_rival_and_the_wrapped_bio() {
        let mut list = blacklist(0, ASPECT);
        list.style = MenuStyle::Rival;
        list.title = "Blacklist 10".into();
        list.entries = vec![rival(10, RivalStatus::InProgress)];
        list.message =
            "He is a rich guy from the coast and he thinks garage custom is an excuse for being broke. ".repeat(3);
        let nodes = built(list);
        let drawn = texts(&nodes);
        assert!(drawn.contains(&"BLACKLIST 10") && drawn.contains(&"RIVAL 10"));
        for label in ["ALIAS", "CAR", "STRENGTH", "BARON", "PORSCHE", "INFRACTIONS"] {
            assert!(drawn.contains(&label), "{label}");
        }
        assert!(
            drawn.iter().filter(|line| line.starts_with("He is") || line.contains("garage")).count() >= 3,
            "wrapped"
        );
        assert_eq!(pictures(&nodes, 1010), 1);
    }

    #[test]
    fn a_status_has_its_own_words_and_colours() {
        assert_eq!(status_text(RivalStatus::InProgress), "IN PROGRESS");
        let colours = [RivalStatus::Complete, RivalStatus::InProgress, RivalStatus::Locked].map(status_colour);
        assert!(colours[0] != colours[1] && colours[1] != colours[2] && colours[0] != colours[2]);
    }
}
