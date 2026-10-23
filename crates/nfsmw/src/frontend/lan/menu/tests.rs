//! The LAN menu driven key by key against the mock network: home, host, join with a password, the lobby following the
//! session, the results page.

use nfsmw_net::client_session::{EndReason, SessionEvent};
use nfsmw_net::message::{EventHead, List, Mode, RaceResults, ResultRow};

use super::*;
use crate::frontend::lan::backend::ServerEntry;
use crate::frontend::lan::mock::{Call, MockNetFrontend};
use crate::frontend::lan::{Disconnect, NetFrontend};
use crate::settings::{Partial, Settings};

fn settings() -> Settings {
    Settings::from(Partial::default())
}

/// Presses `keys` on the menu over `net`, a frame between each.
fn press(menu: &mut LanMenu, net: &mut MockNetFrontend, keys: &[LanKey]) -> Vec<MenuOut> {
    let settings = settings();
    let mut out = Vec::new();
    for key in keys {
        let mut cx = Cx::new(net, &settings, &[]);
        menu.key(*key, &mut cx);
        menu.frame(&mut cx, 1.0 / 60.0);
        out.append(&mut cx.out);
    }
    out
}

fn frame(menu: &mut LanMenu, net: &mut MockNetFrontend) {
    let settings = settings();
    let mut cx = Cx::new(net, &settings, &[]);
    menu.frame(&mut cx, 1.0 / 60.0);
}

fn view(menu: &LanMenu, net: &mut MockNetFrontend) -> PageView {
    let settings = settings();
    let cx = Cx::new(net, &settings, &[]);
    menu.view(&cx)
}

fn typed(text: &str) -> Vec<LanKey> {
    text.chars().map(|c| LanKey::Typed(Typed::Char(c))).collect()
}

fn server(name: &str, password: bool) -> ServerEntry {
    ServerEntry {
        addr: "192.168.1.20:9750".parse().unwrap(),
        name: name.into(),
        mode: Some(Mode::FreeRoam),
        players: 2,
        max_players: 8,
        compatible: true,
        password,
        in_progress: false,
        ping_ms: Some(3),
    }
}

use rows::PageView;

#[test]
fn home_lists_host_join_and_back_and_back_closes() {
    let mut menu = LanMenu::default();
    let mut net = MockNetFrontend::default();
    menu.open(&net.status());
    let page = view(&menu, &mut net);
    assert_eq!(page.title, "LAN Game");
    assert_eq!(page.labels(), ["Host a Game", "Join a Game", "Your name", "Back"]);
    assert_eq!(press(&mut menu, &mut net, &[LanKey::Back]), [MenuOut::Close]);
    assert!(!menu.open);
}

#[test]
fn the_name_row_edits_the_name_and_asks_to_save_it_only_when_it_is_valid() {
    let mut menu = LanMenu::default();
    let mut net = MockNetFrontend::default();
    menu.open(&net.status());
    press(&mut menu, &mut net, &[LanKey::Down, LanKey::Down, LanKey::Accept]);
    assert!(menu.typing());
    assert_eq!(view(&menu, &mut net).row("Your name").unwrap().value, "Player_", "typing starts from the current name");
    let mut keys = vec![LanKey::Typed(Typed::Backspace); 6];
    keys.extend(typed("Ann"));
    assert!(press(&mut menu, &mut net, &keys).is_empty());
    let out = press(&mut menu, &mut net, &[LanKey::Accept]);
    let [MenuOut::SaveName(name)] = out.as_slice() else { panic!("one save: {out:?}") };
    assert_eq!(name.as_str(), "Ann");
    assert!(!menu.typing());
    // An empty name is not a player name: nothing is saved and the row keeps the keys.
    press(&mut menu, &mut net, &[LanKey::Accept]);
    press(&mut menu, &mut net, &vec![LanKey::Typed(Typed::Backspace); 6]);
    assert!(press(&mut menu, &mut net, &[LanKey::Accept]).is_empty());
    assert!(menu.typing());
    // Back drops the edit and the menu stays open.
    assert!(press(&mut menu, &mut net, &[LanKey::Back]).is_empty());
    assert!(!menu.typing() && menu.open);
}

#[test]
fn hosting_sends_the_form_and_the_lobby_opens_when_seated() {
    let mut menu = LanMenu::default();
    menu.set_tracks(vec![city(), TrackChoice { id: 7, title: "Circuit 1.1.1".into(), mode: Some(Mode::Circuit) }]);
    let mut net = MockNetFrontend::default();
    menu.open(&net.status());
    press(&mut menu, &mut net, &[LanKey::Accept]);
    assert_eq!(menu.page(), Page::Host);
    assert_eq!(view(&menu, &mut net).row("Room name").unwrap().value, "Player's game");
    // Mode: Circuit, then the password, then Start Hosting.
    press(&mut menu, &mut net, &[LanKey::Down, LanKey::Right]);
    let mut keys = vec![LanKey::Down; 6];
    keys.push(LanKey::Accept);
    keys.extend(typed("pw"));
    keys.extend([LanKey::Accept, LanKey::Down, LanKey::Accept]);
    press(&mut menu, &mut net, &keys);
    let calls = net.take_calls();
    let Some(Call::Host(options)) = calls.first() else { panic!("hosted: {calls:?}") };
    assert_eq!(options.settings.mode, Mode::Circuit);
    assert_eq!((options.settings.track, options.settings.laps), (7, 1));
    assert_eq!(options.password.as_deref(), Some("pw"));
    assert!(options.settings.password_required);
    assert_eq!(options.loadout.car_type, crate::frontend::lan::car::car_type(crate::scenes::world::DEFAULT_CAR));
    net.lobby = MockNetFrontend::in_lobby(&["Player"], 0).lobby;
    net.status = Some(NetStatus::InLobby);
    frame(&mut menu, &mut net);
    assert_eq!(menu.page(), Page::Lobby);
}

#[test]
fn a_session_seated_from_the_console_opens_the_lobby() {
    let mut menu = LanMenu::default();
    let mut net = MockNetFrontend::in_lobby(&["Me"], 0);
    frame(&mut menu, &mut net);
    assert!(menu.open && menu.visible(&net.status()));
    assert_eq!(menu.page(), Page::Lobby);
}

#[test]
fn a_host_failure_stays_on_the_form_and_says_why() {
    let mut menu = LanMenu::default();
    let mut net = MockNetFrontend { fail: Some("port 9750 is in use".into()), ..MockNetFrontend::default() };
    menu.open(&net.status());
    press(&mut menu, &mut net, &[LanKey::Accept]);
    press(&mut menu, &mut net, &[LanKey::Click(8)]);
    assert_eq!(menu.page(), Page::Host);
    assert_eq!(view(&menu, &mut net).error.as_deref(), Some("port 9750 is in use"));
}

#[test]
fn join_scans_lists_servers_and_asks_a_password_for_a_locked_one() {
    let mut menu = LanMenu::default();
    let mut net = MockNetFrontend::default();
    menu.open(&net.status());
    press(&mut menu, &mut net, &[LanKey::Down, LanKey::Accept]);
    assert_eq!(menu.page(), Page::Browser);
    assert_eq!(net.take_calls(), [Call::Scan]);
    net.servers = vec![server("Open room", false), server("Locked room", true)];
    let page = view(&menu, &mut net);
    assert_eq!(page.labels(), ["Open room", "Locked room", "Refresh", "Direct connect", "Back"]);
    assert_eq!(page.rows[1].value, "Free Roam  2/8  3 ms  locked");
    let mut keys = vec![LanKey::Down, LanKey::Accept];
    keys.extend(typed("hunter2"));
    keys.push(LanKey::Accept);
    press(&mut menu, &mut net, &keys);
    assert_eq!(net.take_calls(), [Call::Connect("192.168.1.20:9750".parse().unwrap(), Some("hunter2".into()))]);
}

#[test]
fn direct_connect_parses_the_typed_address_with_the_settings_port() {
    let mut menu = LanMenu::default();
    let mut net = MockNetFrontend::default();
    menu.open(&net.status());
    press(&mut menu, &mut net, &[LanKey::Click(1)]);
    net.take_calls();
    let mut keys = vec![LanKey::Click(1)];
    keys.extend(typed("10.0.0.9"));
    keys.push(LanKey::Accept);
    press(&mut menu, &mut net, &keys);
    assert_eq!(net.take_calls(), [Call::Connect("10.0.0.9:9750".parse().unwrap(), None)]);
    let mut bad = vec![LanKey::Click(1)];
    bad.extend(typed("lan-box"));
    bad.push(LanKey::Accept);
    press(&mut menu, &mut net, &bad);
    assert!(view(&menu, &mut net).error.unwrap().contains("not an address"));
}

#[test]
fn a_server_that_wants_a_password_brings_the_prompt_back() {
    let mut menu = LanMenu::default();
    let mut net = MockNetFrontend::default();
    menu.open(&net.status());
    press(&mut menu, &mut net, &[LanKey::Click(1)]);
    net.servers = vec![server("Room", false)];
    press(&mut menu, &mut net, &[LanKey::Click(0)]);
    net.status = Some(NetStatus::Disconnected(Disconnect::Session(EndReason::PasswordRequired)));
    frame(&mut menu, &mut net);
    let page = view(&menu, &mut net);
    assert_eq!(page.labels(), ["Password", "Join", "Cancel"]);
    assert_eq!(page.error.as_deref(), Some("this game needs a password"));
}

#[test]
fn a_kick_in_the_lobby_goes_back_to_the_browser_with_the_reason() {
    let mut menu = LanMenu::default();
    let mut net = MockNetFrontend::in_lobby(&["Host", "Me"], 1);
    menu.open(&net.status());
    assert_eq!(menu.page(), Page::Lobby);
    frame(&mut menu, &mut net);
    let kick = nfsmw_net::message::Kick { reason: nfsmw_net::message::KickReason::Kicked, ban_minutes: 0 };
    net.status = Some(NetStatus::Disconnected(Disconnect::Session(EndReason::Kicked(kick))));
    frame(&mut menu, &mut net);
    assert_eq!(menu.page(), Page::Browser);
    assert_eq!(view(&menu, &mut net).error.as_deref(), Some("you were kicked"));
}

#[test]
fn results_open_over_the_world_and_continue_to_the_lobby() {
    let mut menu = LanMenu::default();
    let mut net = MockNetFrontend::in_lobby(&["Host", "Ann"], 0);
    net.status = Some(NetStatus::Playing);
    assert!(!menu.visible(&net.status()));
    let row = |slot, place, time_ms| ResultRow { slot, entity_id: 0, place, time_ms, best_lap_ms: 0, flags: 0 };
    let results = RaceResults {
        head: EventHead::default(),
        rows: List::new(vec![row(1, 1, 60_000), row(0, 2, 61_000)]).unwrap(),
    };
    net.events.push(SessionEvent::Results(results));
    frame(&mut menu, &mut net);
    assert!(menu.visible(&net.status()), "the results show over the world");
    let page = view(&menu, &mut net);
    assert_eq!(page.labels(), ["1st  Ann", "2nd  Host", "Continue"]);
    assert_eq!(menu.feed.last(1).next().unwrap().text, "Ann won (1st of 2)");
    net.status = Some(NetStatus::InLobby);
    press(&mut menu, &mut net, &[LanKey::Accept]);
    assert_eq!(menu.page(), Page::Lobby);
}

#[test]
fn the_car_select_answer_changes_the_loadout_when_seated() {
    let mut menu = LanMenu::default();
    let mut net = MockNetFrontend::in_lobby(&["Me"], 0);
    let settings = settings();
    let mut cx = Cx::new(&mut net, &settings, &[]);
    menu.car_chosen("SL65", &mut cx);
    assert_eq!(menu.car, "SL65");
    let calls = net.take_calls();
    let [Call::Loadout(loadout)] = calls.as_slice() else { panic!("{calls:?}") };
    assert_eq!(loadout.car_type, crate::frontend::lan::car::car_type("SL65"));
}
