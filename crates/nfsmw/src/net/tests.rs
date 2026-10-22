//! The game's client against the real server loop, without the install: the server runs the toy world, the game's
//! `NetClient` (with its `GameCar`, which no world lends anything to) and a scripted bot attach over the listen host's
//! in-memory link, on a fake 60 Hz clock. The client-mode world over the real server is in
//! `scenes/world/net_client/installed.rs`.

use std::thread;
use std::time::Duration;

use blackbox_net_core::endpoint::EndpointConfig;
use blackbox_net_core::secure::IdentityKey;
use nfsmw_net::client_session::SessionConfig;
use nfsmw_net::loadout::Loadout;
use nfsmw_net::message::{Name, SessionPhase};
use nfsmw_net::predict::{InputState, PredictedCar};
use nfsmw_server::bot::{Bot, BotConfig};
use nfsmw_server::{Server, ServerConfig, ServerSetup, toy_build};
use nfsmw_sim::toy::{ToyConfig, ToyWorld};
use nfsmw_sim::{EntityView, PlayerId, WorldSim};

use super::client::NetClient;

/// Real time each tick leaves the net threads.
const TICK_SLEEP: Duration = Duration::from_millis(1);

fn config() -> ServerConfig {
    let mut config = ServerConfig::from_toml("countdown_seconds = 1\nstatus_seconds = 0\n").unwrap();
    config.bind = [127, 0, 0, 1].into();
    config.port = 0;
    config.discovery = false;
    config
}

fn ms(tick: u64) -> u64 {
    tick * 1000 / 60
}

#[test]
fn the_game_client_joins_the_real_server_over_the_listen_host_link() {
    let world = ToyWorld::new(7, ToyConfig { traffic: 2, police: 0, ..ToyConfig::default() });
    let setup = ServerSetup::toy(world.config().car_types);
    let endpoint = nfsmw_server::bind(&config(), IdentityKey::generate().unwrap()).unwrap();
    let mut server = Server::new(config(), setup, endpoint, world, 0).unwrap();
    let key = IdentityKey::generate().unwrap().key_id();
    let link = server.connect_local(key, EndpointConfig::default()).unwrap();
    let build = toy_build();
    let session = SessionConfig::new(Name::new("host").unwrap(), Loadout::stock(0), build.sim_build, build.data_id);
    let mut client = NetClient::new(link, session, true);
    let mut bot: Option<Bot> = None;
    for tick in 0..60 * 8 {
        if bot.is_none() && client.session().slot().is_some() {
            let link = server.connect_local(IdentityKey::generate().unwrap().key_id(), EndpointConfig::default());
            bot = Some(Bot::over(link.unwrap(), BotConfig::new(1, 9)));
        }
        if let Some(bot) = bot.as_mut() {
            bot.step(ms(tick));
        }
        server.tick_once(ms(tick));
        client.set_now_ms(ms(tick));
        client.pump();
        // Neutral input every due tick: no world is lent, so a step leaves the car and every own block corrects it to
        // the server's state.
        client.session_mut().drive(|_| InputState::default());
        thread::sleep(TICK_SLEEP);
    }
    assert_eq!(server.lobby().wire_phase(), SessionPhase::Running, "the client's ready started the session");
    assert_eq!(client.session().slot(), Some(0), "the listen host's player is slot 0, the world's lead");
    assert!(client.session().status().is_joined());
    let car = client.session().own_car().expect("an own-car block placed the car");
    assert!(car.has_pending(), "no world was lent: the server's state waits for the next lend");
    let mut views = Vec::new();
    server.world().entities(&mut views);
    let served = views.iter().find(|view| view.owner == Some(PlayerId(0))).expect("the host's car");
    let apart = distance(served, car.state().position);
    assert!(apart < 2.0, "the car the session holds follows the server's corrections ({apart:.3} m)");
    assert!(client.session().corrections().checks() > 0, "own blocks were compared with the prediction");
    let mut display = Vec::new();
    client.session_mut().remote_entities(&mut display);
    let owners: Vec<_> = display.iter().filter_map(|(_, drawn)| drawn.view.owner).collect();
    assert!(owners.contains(&PlayerId(1)), "the bot's car is a remote entity ({owners:?})");
    assert!(bot.is_some_and(|bot| bot.report().seated));
}

fn distance(view: &EntityView, position: [f32; 3]) -> f32 {
    view.position.iter().zip(position).map(|(a, b)| (a - b) * (a - b)).sum::<f32>().sqrt()
}
