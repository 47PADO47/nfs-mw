//! A race with remote players over the installed world: the server asks for a sprint, the people sit on the grid beside
//! the lead, drive on their seats' pedals, and the clients know their racer cars as their own player cars. Ignored
//! without `NFSMW_GAME_DIR`.

use blackbox_race::Phase;
use nfsmw_data::gameplay::RaceType;
use nfsmw_server::{RaceRequest, WorldControl};
use nfsmw_sim::{EntityKind, EntityView, InputState, PlayerId, SimEvent, SimInputs, WorldSim};

use super::installed::{CAR, join, open};
use crate::scenes::world::net::flags::OWN_PARKED;
use crate::scenes::world::net::host::WorldSimHost;
use nfsmw_net::message::Mode;

const LEAD: PlayerId = PlayerId(0);
const PEOPLE: [PlayerId; 2] = [PlayerId(1), PlayerId(2)];

fn throttle() -> SimInputs {
    let mut inputs = SimInputs::default();
    for id in [LEAD, PEOPLE[0], PEOPLE[1]] {
        inputs.set(id, InputState { throttle: 1.0, ..InputState::default() });
    }
    inputs
}

fn cars(host: &WorldSimHost) -> Vec<EntityView> {
    let mut views = Vec::new();
    host.entities(&mut views);
    views.retain(|view| view.kind == EntityKind::PlayerCar);
    views
}

fn racing(host: &WorldSimHost) -> bool {
    let engine = host.scene().race.as_ref().and_then(|session| session.engine.as_ref());
    engine.is_some_and(|engine| engine.phase() == Phase::Racing)
}

/// A request for the first sprint of the install (no story race) with `players` racing.
fn sprint_request(host: &WorldSimHost, players: Vec<PlayerId>) -> RaceRequest {
    let data = host.scene().gameplay.clone().expect("the race data");
    let sprint = data
        .races
        .iter()
        .find(|race| {
            race.race_type == Some(RaceType::Sprint)
                && !race.story.nis_shell
                && race.start.is_some()
                && !crate::scenes::world::race::is_story_race(race)
        })
        .expect("a sprint race in the install");
    RaceRequest {
        mode: Mode::Sprint,
        track: blackbox_feng::fe_hash_upper(&sprint.name),
        laps: 0,
        traffic_density: 0,
        players,
    }
}

#[test]
#[ignore = "needs the installed game; set NFSMW_GAME_DIR (about 2 minutes)"]
fn remote_players_race_beside_the_lead_and_are_their_own_cars_to_the_clients() {
    let _turn = super::super::super::installed_turn::take();
    let mut host = open(5, Some((0, 0)));
    for id in PEOPLE {
        host.add_player(id, &join(CAR)).unwrap();
    }
    let request = sprint_request(&host, vec![LEAD, PEOPLE[0], PEOPLE[1]]);
    host.start_race(&request).expect("the world starts the race");
    assert_eq!(host.scene().race_people, PEOPLE, "the remote players are the race's people, in grid order");
    for id in PEOPLE {
        assert!(host.scene().players.is_parked(id), "player {} left free roam for the race seat", id.0);
    }

    // The grid stands and waits: the server has not scheduled GO, so nobody counts down.
    let mut events: Vec<SimEvent> = Vec::new();
    assert!(!host.race_staged(), "the racers are still loading");
    for tick in 0..60 * 120 {
        host.step(&throttle(), &mut events);
        if host.race_staged() {
            break;
        }
        assert!(tick < 60 * 119, "the grid never stood");
    }
    let staged = cars(&host);
    for _ in 0..120 {
        host.step(&throttle(), &mut events);
    }
    assert!(host.race_staged() && !racing(&host), "a staged race waits for the server, throttle or not");
    for (first, then) in staged.iter().zip(&cars(&host)) {
        let moved = (0..3).map(|axis| (then.position[axis] - first.position[axis]).powi(2)).sum::<f32>().sqrt();
        assert!(moved < 1.0, "the cars are held on the grid: {moved:.2} m");
    }

    // GO falls on the tick the server scheduled: still counting down just before it, racing just after.
    let go = host.tick + 300;
    host.schedule_race_go(go);
    while host.tick + 5 < go {
        host.step(&throttle(), &mut events);
        assert!(!racing(&host), "GO came early, at tick {} of {go}", host.tick);
    }
    assert!(!racing(&host), "five ticks before GO the countdown still runs");
    while host.tick < go + 5 {
        host.step(&throttle(), &mut events);
    }
    assert!(racing(&host), "five ticks after GO the race runs (tick {} of {go})", host.tick);

    let before = cars(&host);
    let owners: Vec<_> = before.iter().filter_map(|view| view.owner).collect();
    assert_eq!(owners, [LEAD, PEOPLE[0], PEOPLE[1]], "the people's racers are exported as their player cars");
    for _ in 0..60 * 6 {
        host.step(&throttle(), &mut events);
    }
    let after = cars(&host);
    for (first, then) in before.iter().zip(&after) {
        let moved = (0..3).map(|axis| (then.position[axis] - first.position[axis]).powi(2)).sum::<f32>().sqrt();
        let owner = first.owner.map_or(255, |id| id.0);
        assert!(moved > 20.0, "player {owner} drove on its pedals: {moved:.1} m in six seconds");
    }
    for id in PEOPLE {
        let own = host.own_state(id).expect("an own-car block");
        assert!(own.rpm > 500.0, "player {} has an engine speed, {} rpm", id.0, own.rpm);
        assert!(own.gear >= 2, "player {} is in a forward gear: wire gear {}", id.0, own.gear);
        assert_eq!(own.flags & OWN_PARKED, 0, "player {} is in the world, not parked", id.0);
    }

    let states = crate::scenes::world::net::sim::racer_states(host.scene()).expect("a race engine");
    let people: Vec<_> = states.iter().filter_map(|racer| racer.player).collect();
    assert_eq!(people, [LEAD, PEOPLE[0], PEOPLE[1]], "the race events name the people by their player ids");

    // The race ends the moment it is over: a session has no race-over message or post-race screen to wait for.
    host.scene_mut().0.knock_out_every_human();
    for _ in 0..3 {
        host.step(&throttle(), &mut events);
    }
    assert!(
        host.scene().race.is_none() && host.scene().race_people.is_empty(),
        "the race is down three ticks after it ended"
    );
    for id in PEOPLE {
        assert!(!host.scene().players.is_parked(id), "player {} is back on the road at once", id.0);
    }

    host.stop_race();
    assert!(host.scene().race.is_none() && host.scene().race_people.is_empty());
    for id in PEOPLE {
        assert!(!host.scene().players.is_parked(id), "player {} is back on the road", id.0);
    }
}

#[test]
#[ignore = "needs the installed game; set NFSMW_GAME_DIR (about 1 minute)"]
fn an_idle_lead_is_a_computer_racer_the_race_does_not_wait_for() {
    let _turn = super::super::super::installed_turn::take();
    let mut host = open(6, Some((0, 0)));
    for id in PEOPLE {
        host.add_player(id, &join(CAR)).unwrap();
    }
    // A dedicated server: nobody took slot 0, so the request names the people alone.
    let request = sprint_request(&host, PEOPLE.to_vec());
    host.start_race(&request).expect("the world starts the race");
    let mut events: Vec<SimEvent> = Vec::new();
    for tick in 0..60 * 120 {
        host.step(&SimInputs::default(), &mut events);
        if host.race_staged() {
            break;
        }
        assert!(tick < 60 * 119, "the grid never stood");
    }
    let engine = host.scene().race.as_ref().and_then(|session| session.engine.as_ref()).expect("a race engine");
    let humans: Vec<bool> = engine.racers().iter().map(|racer| racer.human).take(3).collect();
    assert_eq!(humans, [false, true, true], "the idle lead is a computer racer, the people are the humans");
    let states = crate::scenes::world::net::sim::racer_states(host.scene()).expect("a race engine");
    let seated: Vec<_> = states.iter().filter_map(|racer| racer.player).collect();
    assert_eq!(seated, PEOPLE, "the race events name the people, not the idle lead");

    // The people are done and the race is over at once; nothing waits for the lead.
    let go = host.tick + 200;
    host.schedule_race_go(go);
    while host.tick < go + 5 {
        host.step(&SimInputs::default(), &mut events);
    }
    host.scene_mut().0.knock_out_every_human();
    for _ in 0..3 {
        host.step(&SimInputs::default(), &mut events);
    }
    assert!(host.scene().race.is_none(), "the race ended with the people, not 30 s after");
    for id in PEOPLE {
        assert!(!host.scene().players.is_parked(id), "player {} is back on the road", id.0);
    }
}
