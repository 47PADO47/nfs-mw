//! The players' cars report the pairs that touched in a tick, with how fast they closed (`players/contact.rs`), for a
//! server's contact events. The example car is 4.4 m long, so cars 4.2 m apart overlap by 0.2 m.

use super::*;
use crate::scenes::world::players::{Party, PlayerId, TickContact, collide_players};

/// A car at `z` (physics space) moving along z at `speed`, coming from a third of a metre back along its way.
fn car_at(z: f32, speed: f32) -> CarSim {
    let mut car = sim();
    let at = Vec3::new(0.0, 0.0, z);
    car.body_mut().place(at, Quat::IDENTITY);
    car.body_mut().linear_velocity = Vec3::Z * speed;
    car.previous_contact_pose = (at - Vec3::Z * speed.signum() / 3.0, Quat::IDENTITY);
    car
}

#[test]
fn two_players_that_overlap_and_close_are_one_contact() {
    let mut lead = car_at(0.0, 10.0);
    let mut guest = car_at(4.2, -10.0);
    let mut far = car_at(200.0, 0.0);
    let mut contacts = Vec::new();
    let ids = [PlayerId(2), PlayerId(5)];
    collide_players(Some((PlayerId(0), &mut lead)), &mut [&mut guest, &mut far], &ids, &mut contacts);
    assert_eq!(contacts, [TickContact { a: Party::Player(PlayerId(0)), b: Party::Player(PlayerId(2)), speed: 20.0 }]);
}

#[test]
fn guests_report_their_pairs_by_their_ids() {
    let mut a = car_at(0.0, 0.0);
    let mut b = car_at(4.2, -4.0);
    let mut contacts = Vec::new();
    collide_players(None, &mut [&mut a, &mut b], &[PlayerId(3), PlayerId(9)], &mut contacts);
    assert_eq!(contacts.len(), 1);
    assert_eq!((contacts[0].a, contacts[0].b), (Party::Player(PlayerId(3)), Party::Player(PlayerId(9))));
    assert!((contacts[0].speed - 4.0).abs() < 1e-5);
}
