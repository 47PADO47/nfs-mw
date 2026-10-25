//! `NetSession`'s side of the Discord party and invites: what the party shows, and which invites are taken.

use nfsmw_net::client_session::LobbyView;
use nfsmw_server::ServerConfig;

use super::*;

fn session() -> NetSession {
    NetSession::new(None, PlayerSetup { name: "Test".into(), car: "BMWM3GTR".into() })
}

/// A lobby this client is seated in, with the server's default settings.
fn seated_lobby() -> LobbyView {
    let mut lobby = LobbyView::new(8);
    lobby.my_slot = Some(1);
    lobby.settings = Some(ServerConfig::from_toml("").unwrap().settings);
    lobby
}

#[test]
fn a_session_with_no_seat_has_no_party() {
    let mut session = session();
    assert_eq!(session.lan_game(None), None);
    session.mode = Mode::Join;
    session.joined = Some("10.0.0.7:9750".parse().unwrap());
    assert_eq!(session.lan_game(None), None, "joining, not yet seated");
}

#[test]
fn a_joined_game_shows_its_server_and_is_not_offered_on() {
    let mut session = session();
    session.mode = Mode::Join;
    session.joined = Some("10.0.0.7:9750".parse().unwrap());
    session.lobby = Some(seated_lobby());
    let game = session.lan_game(Some("192.168.1.5".parse().unwrap())).expect("seated");
    assert_eq!(game.host, "10.0.0.7:9750".parse().unwrap());
    assert!(!game.open, "the host offers the game, not its guests");
}

#[test]
fn an_invite_is_ignored_in_a_seat_or_while_hosting_and_taken_otherwise() {
    let invited = "127.0.0.1:9750".parse().unwrap();
    let mut seated = session();
    seated.mode = Mode::Join;
    seated.joined = Some("10.0.0.7:9750".parse().unwrap());
    seated.lobby = Some(seated_lobby());
    seated.join_from_discord(invited);
    assert_eq!(seated.joined, Some("10.0.0.7:9750".parse().unwrap()), "a seated player stays");

    let mut hosting = session();
    hosting.mode = Mode::Host;
    hosting.join_from_discord(invited);
    assert_eq!((hosting.mode, hosting.joined), (Mode::Host, None), "a host stays");

    let mut idle = session();
    idle.join_from_discord(invited);
    assert_eq!((idle.mode, idle.joined), (Mode::Join, Some(invited)), "a free player joins the invited server");
}
