//! The lobby page against the mock network: the roster's badges, ready and role, chat, quick lines, votes, the player
//! menu, the host's settings and start, the countdown.

use nfsmw_net::lobby::{FLAG_ADMIN, FLAG_MUTED};
use nfsmw_net::message::{AdminCommand, List, StartCountdown, VoteOption};
use nfsmw_net::roles::LocalMutes;

use super::super::player::PlayerMenu;
use super::*;
use crate::frontend::lan::NetFrontend;
use crate::frontend::lan::mock::{Call, MockNetFrontend};
use crate::settings::{Partial, Settings};

fn settings() -> Settings {
    Settings::from(Partial::default())
}

fn labels(page: &LobbyPage, net: &MockNetFrontend) -> Vec<String> {
    page.rows(net.lobby().unwrap(), &[], "BMWM3GTR").into_iter().map(|row| row.label).collect()
}

/// Selects the row called `label` and accepts it.
fn accept(page: &mut LobbyPage, net: &mut MockNetFrontend, label: &str) -> Go {
    let at = labels(page, net).iter().position(|l| l == label).unwrap_or_else(|| panic!("no row {label}"));
    page.cursor = Cursor(at);
    let settings = settings();
    let mut cx = Cx::new(net, &settings, &[]);
    page.accept(&mut cx)
}

#[test]
fn the_roster_shows_badges_car_ready_and_ping() {
    let mut net = MockNetFrontend::in_lobby(&["Host", "Ann", "Bob"], 1);
    let lobby = net.lobby.as_mut().unwrap();
    lobby.players[1].ready = true;
    lobby.players[2].flags = FLAG_ADMIN | FLAG_MUTED;
    lobby.players[2].car_type = crate::frontend::lan::car::car_type("SL65");
    let rows = LobbyPage::default().rows(net.lobby().unwrap(), &["SL65".to_owned()], "BMWM3GTR");
    assert_eq!(rows[0].label, "Host  [host]");
    assert_eq!(rows[1].label, "Ann  (you)");
    assert_eq!(rows[1].value, "-  ready  12 ms");
    assert_eq!(rows[2].label, "Bob  [admin]  [muted]");
    assert_eq!(rows[2].value, "SL65  not ready  12 ms");
    let names = labels(&LobbyPage::default(), &net);
    assert_eq!(names[3..], ["Ready", "Car", "Role", "Say", "Quick message", "Vote to restart", "Leave"]);
}

#[test]
fn ready_role_and_car_send_or_ask() {
    let mut net = MockNetFrontend::in_lobby(&["Host", "Me"], 1);
    let mut page = LobbyPage::default();
    accept(&mut page, &mut net, "Ready");
    accept(&mut page, &mut net, "Role");
    assert_eq!(net.take_calls(), [Call::Ready(true), Call::Role(nfsmw_net::message::Role::Spectator)]);
    let settings = settings();
    page.cursor = Cursor(labels(&page, &net).iter().position(|l| l == "Car").unwrap());
    let mut cx = Cx::new(&mut net, &settings, &[]);
    page.accept(&mut cx);
    assert_eq!(cx.out, [MenuOut::CarSelect]);
}

#[test]
fn chat_types_then_sends_and_quick_lines_cycle() {
    let mut net = MockNetFrontend::in_lobby(&["Host", "Me"], 1);
    let mut page = LobbyPage::default();
    accept(&mut page, &mut net, "Say");
    assert!(page.typing());
    for c in "gg all".chars() {
        page.take(Typed::Char(c));
    }
    accept(&mut page, &mut net, "Say");
    assert!(!page.typing() && page.chat.text.is_empty());
    let settings = settings();
    page.cursor = Cursor(labels(&page, &net).iter().position(|l| l == "Quick message").unwrap());
    let mut cx = Cx::new(&mut net, &settings, &[]);
    page.step(&mut cx, false);
    page.accept(&mut cx);
    assert_eq!(net.take_calls(), [Call::Say("gg all".into()), Call::Quick(7)]);
}

#[test]
fn only_the_host_or_an_admin_gets_settings_and_start_and_start_waits_for_ready() {
    let mut player = MockNetFrontend::in_lobby(&["Host", "Me"], 1);
    assert!(!labels(&LobbyPage::default(), &player).contains(&"Start race".to_owned()));
    player.lobby.as_mut().unwrap().players[1].flags = FLAG_ADMIN;
    assert!(labels(&LobbyPage::default(), &player).contains(&"Session settings".to_owned()));
    let mut host = MockNetFrontend::in_lobby(&["Me", "Ann"], 0);
    let mut page = LobbyPage::default();
    assert_eq!(accept(&mut page, &mut host, "Start race"), Go::Stay);
    assert!(host.take_calls().is_empty(), "not everybody is ready");
    for row in &mut host.lobby.as_mut().unwrap().players {
        row.ready = true;
    }
    accept(&mut page, &mut host, "Start race");
    assert_eq!(host.take_calls(), [Call::Start]);
    assert_eq!(accept(&mut page, &mut host, "Session settings"), Go::Settings);
    assert_eq!(accept(&mut page, &mut host, "Ann"), Go::Player(1));
    assert_eq!(accept(&mut page, &mut host, "Me  [host]  (you)"), Go::Stay, "no menu for yourself");
}

#[test]
fn a_running_vote_lists_its_options_to_vote_for() {
    let mut net = MockNetFrontend::in_lobby(&["Host", "Ann", "Me"], 2);
    net.lobby.as_mut().unwrap().vote = Some(nfsmw_net::message::VoteState {
        kind: VoteKind::Kick,
        options: List::new(vec![VoteOption { id: 1, count: 2 }]).unwrap(),
        ends_tick: 900,
    });
    let mut page = LobbyPage::default();
    let rows = page.rows(net.lobby().unwrap(), &[], "-");
    let vote = rows.iter().find(|row| row.label == "Vote: kick Ann").unwrap();
    assert_eq!(vote.value, "2 votes");
    accept(&mut page, &mut net, "Vote: kick Ann");
    assert_eq!(net.take_calls(), [Call::Vote(VoteKind::Kick, 1)]);
}

#[test]
fn leave_leaves() {
    let mut net = MockNetFrontend::in_lobby(&["Host", "Me"], 1);
    let mut page = LobbyPage::default();
    assert_eq!(accept(&mut page, &mut net, "Leave"), Go::Left);
    assert_eq!(net.take_calls(), [Call::Leave]);
}

#[test]
fn the_countdown_counts_to_the_start_tick() {
    let mut net = MockNetFrontend::in_lobby(&["Host"], 0);
    let lobby = net.lobby.as_mut().unwrap();
    lobby.countdown = Some(StartCountdown { start_tick: 600, seconds: 5 });
    assert_eq!(countdown_text(lobby, Some(421.0)), Some("Race starts in 3".into()));
    assert_eq!(countdown_text(lobby, None), Some("Race starts in 5".into()));
    lobby.countdown = None;
    assert_eq!(countdown_text(lobby, Some(0.0)), None);
}

#[test]
fn the_player_menu_mutes_locally_votes_and_for_the_host_kicks_mutes_and_bans() {
    let mut net = MockNetFrontend::in_lobby(&["Me", "Ann"], 0);
    let mut mutes = LocalMutes::new();
    let mut menu = PlayerMenu::new(1);
    let rows: Vec<String> = menu.rows(net.lobby().unwrap(), &mutes).into_iter().map(|r| r.label).collect();
    assert_eq!(rows, ["Mute for me", "Vote to kick", "Kick", "Mute in session", "Ban for 10 minutes", "Back"]);
    let settings = settings();
    for at in 0..5 {
        menu.cursor = Cursor(at);
        let mut cx = Cx::new(&mut net, &settings, &[]);
        menu.accept(&mut cx, &mut mutes);
    }
    assert!(!mutes.shows([1; 16]), "Ann is muted on this machine");
    assert_eq!(
        net.take_calls(),
        [
            Call::Vote(VoteKind::Kick, 1),
            Call::Admin(AdminCommand::Kick { slot: 1 }),
            Call::Admin(AdminCommand::Mute { slot: 1, muted: true }),
            Call::Admin(AdminCommand::Ban { slot: 1, minutes: 10 }),
        ]
    );
    let guest = MockNetFrontend::in_lobby(&["Host", "Me", "Ann"], 1);
    let rows = PlayerMenu::new(2).rows(guest.lobby().unwrap(), &mutes);
    assert_eq!(rows.len(), 3, "a player gets mute, vote and back");
}
