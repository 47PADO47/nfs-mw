//! The host's events over the installed world: a scripted bump between two players and a chase started and stopped
//! from the console. Ignored without `NFSMW_GAME_DIR`.

use glam::Vec3;
use nfsmw_sim::{InputState, PlayerId, SimEvent, SimInputs, WorldSim};

use super::installed::{CAR, join, open};
use crate::scenes::world::ai::pursuit_command;
use crate::scenes::world::road::{self, Spawn};

const LEAD: PlayerId = PlayerId(0);
const GUEST: PlayerId = PlayerId(1);

/// Steps `ticks` ticks with `inputs`, gathering the events.
fn run(host: &mut super::super::host::WorldSimHost, ticks: u32, inputs: &SimInputs) -> Vec<SimEvent> {
    let mut events = Vec::new();
    for _ in 0..ticks {
        host.step(inputs, &mut events);
    }
    events
}

#[test]
#[ignore = "needs the installed game; set NFSMW_GAME_DIR (about 30 s)"]
fn a_bump_between_two_players_is_a_contact_event() {
    let mut host = open(3, Some((0, 0)));
    host.add_player(GUEST, &join(CAR)).unwrap();
    // The guest stands still on the road 9 m in front of the lead, facing the same way. It is put there before its
    // first placement: a car put back on the road after one does not collide for a while (the reset grace).
    let (scene, _) = host.scene_mut();
    let lead = scene.players.lead().unwrap();
    let (position, heading) = (lead.position(), lead.heading());
    let ahead = position + Vec3::new(heading.cos(), heading.sin(), 0.0) * 9.0;
    let collision = scene.residency.collision();
    let road = road::probe(collision, ahead.x, ahead.y, ahead.z + 2.0).expect("the road goes on ahead of the lead");
    let spot = Spawn { position: Vec3::new(ahead.x, ahead.y, road.height), heading };
    let guest = scene.players.get_mut(GUEST).unwrap();
    assert!(guest.spawn(spot, collision, &scene.physics.surfaces), "the guest stands on the road at {spot:?}");
    let mut inputs = SimInputs::default();
    inputs.set(LEAD, InputState { throttle: 1.0, ..InputState::default() });
    let events = run(&mut host, 180, &inputs);
    let contacts: Vec<_> = events.iter().filter(|event| matches!(event, SimEvent::Contact { .. })).collect();
    assert!(!contacts.is_empty(), "the lead runs into the guest: {events:?}");
    for contact in contacts {
        let SimEvent::Contact { a, b, speed } = *contact else { continue };
        assert!(a < b && speed.is_finite() && speed >= 0.0);
    }
    let players = |event: &&SimEvent| matches!(event, SimEvent::Contact { a: 0, b: 1, .. });
    assert!(events.iter().any(|event| players(&event)), "the two players' cars touched: {events:?}");
}

#[test]
#[ignore = "needs the installed game; set NFSMW_GAME_DIR (about 30 s)"]
fn a_chase_started_and_stopped_from_the_console_starts_and_ends_the_leads_pursuit() {
    let mut host = open(4, Some((8, 0)));
    let (scene, renderer) = host.scene_mut();
    pursuit_command(scene, renderer, &["2"]).unwrap();
    let events = run(&mut host, 60, &SimInputs::default());
    assert!(events.contains(&SimEvent::PursuitStarted { player: LEAD }), "{events:?}");
    assert_eq!(events.iter().filter(|event| matches!(event, SimEvent::PursuitStarted { .. })).count(), 1);
    let (scene, renderer) = host.scene_mut();
    pursuit_command(scene, renderer, &["off"]).unwrap();
    let events = run(&mut host, 2, &SimInputs::default());
    assert_eq!(events.iter().filter(|event| matches!(event, SimEvent::PursuitEnded { .. })).count(), 1, "{events:?}");
    assert!(events.contains(&SimEvent::PursuitEnded { player: LEAD }));
    assert!(!events.iter().any(|event| matches!(event, SimEvent::Busted { .. } | SimEvent::Evaded { .. })));
}
