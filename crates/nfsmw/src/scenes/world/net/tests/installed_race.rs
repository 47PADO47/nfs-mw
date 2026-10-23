//! A race with remote players over the installed world: the server asks for a sprint, the people sit on the grid beside
//! the lead, drive on their seats' pedals, and the clients know their racer cars as their own player cars. Ignored
//! without `NFSMW_GAME_DIR`.

use blackbox_race::Phase;
use nfsmw_data::gameplay::RaceType;
use nfsmw_server::{RaceRequest, WorldControl};
use nfsmw_sim::{EntityKind, EntityView, InputState, PlayerId, SimEvent, SimInputs, WorldSim};

use super::installed::{CAR, join, open};
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

#[test]
#[ignore = "needs the installed game; set NFSMW_GAME_DIR (about 2 minutes)"]
fn remote_players_race_beside_the_lead_and_are_their_own_cars_to_the_clients() {
    let mut host = open(5, Some((0, 0)));
    for id in PEOPLE {
        host.add_player(id, &join(CAR)).unwrap();
    }
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
    let request = RaceRequest {
        mode: Mode::Sprint,
        track: blackbox_feng::fe_hash_upper(&sprint.name),
        laps: 0,
        traffic_density: 0,
        players: vec![LEAD, PEOPLE[0], PEOPLE[1]],
    };
    host.start_race(&request).expect("the world starts the race");
    assert_eq!(host.scene().race_people, PEOPLE, "the remote players are the race's people, in grid order");
    for id in PEOPLE {
        assert!(host.scene().players.is_parked(id), "player {} left free roam for the race seat", id.0);
    }

    let mut events: Vec<SimEvent> = Vec::new();
    for tick in 0..60 * 120 {
        host.step(&throttle(), &mut events);
        if racing(&host) {
            break;
        }
        assert!(tick < 60 * 119, "the race never reached the green light");
    }
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

    let states = crate::scenes::world::net::sim::racer_states(host.scene()).expect("a race engine");
    let people: Vec<_> = states.iter().filter_map(|racer| racer.player).collect();
    assert_eq!(people, [LEAD, PEOPLE[0], PEOPLE[1]], "the race events name the people by their player ids");

    host.stop_race();
    assert!(host.scene().race.is_none() && host.scene().race_people.is_empty());
    for id in PEOPLE {
        assert!(!host.scene().players.is_parked(id), "player {} is back on the road", id.0);
    }
}
