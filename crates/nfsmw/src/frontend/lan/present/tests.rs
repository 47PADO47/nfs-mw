use glam::Vec2;

use super::*;
use crate::frontend::lan::hud::SpectatorPanel;
use crate::frontend::lan::menu::rows::Row;
use crate::ui::theme::remastered::menu::layout::Side;

fn view() -> PageView {
    PageView {
        title: "Host a game".into(),
        subtitle: "Status line".into(),
        rows: vec![
            Row::text("Room name", "Ann's game_", true),
            Row::choice("Mode", "Circuit", true),
            Row::choice("Laps", "3", false),
            Row::info("Ann", "ready"),
            Row::action("Start Hosting", true),
            Row::info("Nobody", ""),
        ],
        selected: 1,
        side_title: "Chat".into(),
        side: vec!["Ann: hi".into()],
        error: Some("no port".into()),
        hints: vec![
            (HintKey::Enter, HintDoes::SelectOrType),
            (HintKey::Esc, HintDoes::Back),
            (HintKey::Arrows, HintDoes::Move),
        ],
    }
}

#[test]
fn rows_become_entries_with_the_value_their_kind_shows() {
    let page = page_of(&view(), 0, 16.0 / 9.0);
    let values: Vec<MenuValue> = page.list.entries.iter().map(|entry| entry.value.clone()).collect();
    assert_eq!(
        values,
        [
            MenuValue::Choice { text: "Ann's game_".into(), arrows: false },
            MenuValue::Choice { text: "Circuit".into(), arrows: true },
            MenuValue::Choice { text: "3".into(), arrows: false },
            MenuValue::Choice { text: "ready".into(), arrows: false },
            MenuValue::None,
            MenuValue::None,
        ],
        "a typed text and a line of information have no chevrons, a disabled choice neither, an action has no value"
    );
    assert!(!page.list.entries[2].enabled);
    assert_eq!(
        (page.list.title.as_str(), page.subtitle.as_str(), page.error.as_deref()),
        ("Host a game", "Status line", Some("no port"))
    );
    assert_eq!((page.side_title.as_str(), page.side.len()), ("Chat", 1));
    assert_eq!((page.list.selected, page.list.first, page.list.style), (1, 0, MenuStyle::Sub));
}

#[test]
fn the_footer_hints_are_key_caps_and_the_select_and_back_ones_can_be_clicked() {
    let page = page_of(&view(), 0, 16.0 / 9.0);
    let hints = &page.list.hints;
    assert_eq!(hints[0].keys, [Prompt::Key("Enter".into())]);
    assert_eq!(
        (hints[0].action, hints[1].action, hints[2].action),
        (Some(Action::MenuAccept), Some(Action::MenuBack), None)
    );
}

#[test]
fn a_click_lands_on_the_row_and_the_side_the_themes_draw() {
    let view = view();
    let page = page_of(&view, 0, 16.0 / 9.0);
    let map = ClickMap::of(&view, &page);
    let left = place::left_edge(16.0 / 9.0);
    let on = |row: usize, along: f32| Vec2::new(left + along, place::row_y(row));
    assert_eq!(map.row_at(on(1, 100.0)), Some((1, Side::Middle)));
    assert_eq!(map.row_at(on(1, 330.0)), Some((1, Side::Left)));
    assert_eq!(map.row_at(on(1, 480.0)), Some((1, Side::Right)));
    assert_eq!(map.row_at(Vec2::new(left + 100.0, place::row_y(7) + 200.0)), None, "below the rows there is no row");
    assert_eq!(map.choices, [false, true, true, false, false, false]);
}

#[test]
fn the_window_of_rows_follows_the_selection() {
    assert_eq!(window_first(0, 3, 20), 0);
    assert_eq!(window_first(0, 9, 20), 2);
    assert_eq!(window_first(5, 2, 20), 2);
    assert_eq!(window_first(9, 10, 12), 4, "a list that shrank keeps its last rows in view");
}

#[test]
fn the_hud_keeps_its_lines_their_kind_and_the_spectator_panel() {
    let hud = HudView {
        feed: vec![(FeedKind::Chat, "Ann: hi".into(), 1.0), (FeedKind::Notice, "Bob left".into(), 0.4)],
        vote: Some("Vote: restart".into()),
        countdown: Some("Race starts in 3".into()),
        spectator: Some(SpectatorPanel { following: "Following Ann".into(), board: vec!["1 Ann".into()] }),
    };
    let lan = hud_of(&hud, 1.5);
    assert_eq!(lan.aspect, 1.5);
    assert_eq!(
        lan.feed.iter().map(|line| (line.kind, line.opacity)).collect::<Vec<_>>(),
        [(LanLineKind::Chat, 1.0), (LanLineKind::Notice, 0.4)]
    );
    assert_eq!(lan.countdown.as_deref(), Some("Race starts in 3"));
    assert_eq!(lan.spectator.map(|panel| panel.following), Some("Following Ann".into()));
}
