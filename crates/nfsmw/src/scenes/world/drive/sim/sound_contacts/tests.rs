use super::*;
use crate::audio::ContactSource;
use crate::scenes::world::{drive::DriveInput, road::Spawn};
use blackbox_vehicle::{FlatGround, VehicleSpec};
use glam::{Quat, Vec3};
use nfsmw_data::car::physics::{CarBounds, CarPhysics, WallSpec};

fn car(profile: u32, player: bool) -> CarSim {
    let spec = VehicleSpec::example();
    let mut sim = CarSim::new(
        CarPhysics {
            effect_source: profile,
            bounds: CarBounds { half_dimensions: spec.dimension, pivot: Vec3::ZERO },
            spec,
            tire_integration: Default::default(),
            jump_config: Default::default(),
            walls: WallSpec::default(),
            geometry: None,
            events: None,
            reactions: None,
            bike: None,
        },
        [0.2; 4],
    );
    if !player {
        sim.configure_ai();
    }
    sim
}

#[test]
fn non_player_ground_callbacks_survive_discarding_the_step_return() {
    let ground = FlatGround::new(0.0);
    let mut sim = car(111, false);
    assert!(sim.place_moving(&ground, Spawn { position: Vec3::ZERO, heading: 0.0 }, 0.0));
    sim.vehicle.body_mut().place(Vec3::Y * 0.1, Quat::from_rotation_z(std::f32::consts::FRAC_PI_2));
    sim.vehicle.body_mut().linear_velocity = Vec3::NEG_Y * 10.0;
    let mut heard = false;
    for _ in 0..60 {
        drop(sim.step(&DriveInput::default(), &ground, None));
        let contacts = sim.take_owned_sound_contacts();
        assert!(contacts.iter().all(|c| c.vehicle.is_some_and(|v| v.profile == 111)));
        heard |= contacts.iter().any(|c| c.source == ContactSource::Ground && c.delta_v > 0.0);
        assert!(sim.take_owned_sound_contacts().is_empty());
    }
    assert!(heard, "a real landing must publish an owned callback without a player sound feed");
}

fn pair(a: &mut CarSim, b: &mut CarSim) {
    a.vehicle.body_mut().place(Vec3::Y, Quat::IDENTITY);
    b.vehicle.body_mut().place(Vec3::new(0.0, 1.0, 3.5), Quat::IDENTITY);
    a.vehicle.body_mut().linear_velocity = Vec3::Z * 15.0;
    assert!(a.collide_with(b).is_some(), "exercise the actual two-body solver");
}

#[test]
fn non_player_pair_endpoints_keep_distinct_bodies_even_for_the_same_vehicle_type() {
    let (mut a, mut b) = (car(111, false), car(111, false));
    pair(&mut a, &mut b);
    let (a, b) = (a.take_owned_sound_contacts(), b.take_owned_sound_contacts());
    assert_eq!((a.len(), b.len()), (1, 1));
    assert_ne!(a[0].vehicle.unwrap().identity, b[0].vehicle.unwrap().identity);
    assert_eq!(a[0].vehicle.unwrap().profile, b[0].vehicle.unwrap().profile);
    assert_eq!(a[0].normal, -b[0].normal);
    assert_eq!(a[0].velocity, -b[0].velocity);
    assert!(a[0].delta_v > 0.0 && b[0].delta_v > 0.0);
}

#[test]
fn player_endpoint_is_not_duplicated_when_traffic_is_drained() {
    let (mut player, mut npc) = (car(112, true), car(113, false));
    pair(&mut player, &mut npc);
    let player_contacts = player.take_sound_contacts();
    let npc_contacts = npc.take_owned_sound_contacts();
    assert_eq!((player_contacts.len(), npc_contacts.len()), (1, 1));
    assert_eq!(player_contacts[0].vehicle, None);
    assert_eq!(npc_contacts[0].vehicle.unwrap().profile, 113);
    assert!(npc.take_owned_sound_contacts().is_empty());
}

#[test]
fn immobile_vehicle_sound_keeps_the_notification_without_a_physical_response() {
    let (mut heavy, mut mobile) = (car(114, false), car(115, false));
    let mut spec = heavy.vehicle.spec().clone();
    spec.body.objects.immobile = true;
    heavy.vehicle.set_spec(spec);
    pair(&mut heavy, &mut mobile);
    assert_eq!(heavy.body().linear_velocity, Vec3::Z * 15.0);
    assert!(mobile.body().linear_velocity.length() > 0.0);
    let sounds = heavy.take_owned_sound_contacts();
    assert_eq!(sounds.len(), 1);
    assert!(sounds[0].delta_v > 0.0, "audio uses the callback, not the immobile body's zero response");
}
