//! Actual NPC body pair -> owned callback -> authored graph, with identical background SFX cancelled.
use super::*;
use crate::audio::{Audio, CarSoundState, CarSoundTick, Volumes};
use blackbox_carsound::CarInput;
use game_install::GameDir;
use glam::{Quat, Vec3};
use nfsmw_data::car::physics::PhysicsData;

fn render(dir: GameDir, contacts: Vec<CarContact>) -> Vec<f32> {
    let mut audio = Audio::replay(dir, 48_000);
    audio.set_volumes(Volumes { engine: 0.0, music: 0.0, sfx: 1.0, master: 1.0, hud: 0.0 });
    audio.render_replay(24_000); // Finish volume commands before any engine voice starts.
    let input = CarInput::default();
    let mut state = CarSoundState {
        telemetry: Default::default(),
        car: "BMWM3GTR".into(),
        upgrades: Default::default(),
        input,
        events: vec![],
        scrape: None,
        surface: 0,
        ticks: vec![CarSoundTick { telemetry: Default::default(), input, contacts, surface: 0 }],
        view: Default::default(),
    };
    let mut recorded = Vec::new();
    for _ in 0..180 {
        audio.drive_car(Some(&state), 1.0 / 60.0);
        state.ticks = vec![CarSoundTick { telemetry: Default::default(), input, contacts: vec![], surface: 0 }];
        for &sample in audio.render_replay(800) {
            assert!(sample.is_finite());
            recorded.push(sample);
        }
    }
    recorded
}

fn difference(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len());
    let sum = a.iter().zip(b).map(|(&a, &b)| f64::from(a - b).powi(2)).sum::<f64>();
    (sum / a.len() as f64).sqrt() as f32
}

fn strike(a: &mut CarSim, b: &mut CarSim) {
    // Installed compound bodywork can be much shorter than the enclosing root box.
    // Approach from clear separation until the actual authored shapes resolve.
    let distance = a.collision_radius() + b.collision_radius() + 1.0;
    for step in 0..2048 {
        let gap = distance * (1.0 - step as f32 / 2048.0);
        a.body_mut().place(Vec3::Y * 2.0, Quat::IDENTITY);
        b.body_mut().place(Vec3::new(0.0, 2.0, gap), Quat::IDENTITY);
        a.previous_contact_pose = (a.body().position, a.body().orientation);
        b.previous_contact_pose = (b.body().position, b.body().orientation);
        a.body_mut().linear_velocity = Vec3::Z * 35.0;
        b.body_mut().linear_velocity = Vec3::ZERO;
        if a.collide_with(b).is_some() {
            return;
        }
    }
    panic!("the actual installed bodywork must resolve an approaching collision");
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR), renders SFX offscreen"]
fn installed_npc_vehicle_collisions_reach_authored_audio_without_player_contacts() {
    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").expect("set NFSMW_GAME_DIR")).unwrap();
    let data = PhysicsData::load(&dir).unwrap();
    let background = render(dir.clone(), vec![]);
    for name in ["traf4dseda", "copmidsize", "semia", "trailerb"] {
        let physics = data.car(name).unwrap();
        let profile = physics.effect_source;
        let mut a = CarSim::new(physics, [0.2; 4]);
        // Two authored immobile endpoints need not react. Use an ordinary mobile
        // traffic partner so the heavy endpoint receives its real notification.
        let partner = data.car("traf4dseda").unwrap();
        let partner_profile = partner.effect_source;
        let mut b = CarSim::new(partner, [0.2; 4]);
        a.configure_ai();
        b.configure_ai();
        strike(&mut a, &mut b);
        let mut contacts = a.take_owned_sound_contacts();
        contacts.extend(b.take_owned_sound_contacts());
        assert_eq!(contacts.len(), 2, "{name}: both owner callbacks are delivered");
        assert_eq!(contacts[0].vehicle.unwrap().profile, profile);
        assert_eq!(contacts[1].vehicle.unwrap().profile, partner_profile);
        assert_ne!(contacts[0].vehicle, contacts[1].vehicle);
        assert!(a.take_owned_sound_contacts().is_empty() && b.take_owned_sound_contacts().is_empty());
        let mut missing = contacts.clone();
        for contact in &mut missing {
            contact.vehicle.as_mut().unwrap().profile = u32::MAX;
        }
        assert!(
            difference(&background, &render(dir.clone(), missing)) < 1e-6,
            "a missing vehicle source must not borrow the player's sounds"
        );
        // Isolate the named NPC endpoint; its partner must not mask a lost callback.
        let rms = difference(&background, &render(dir.clone(), vec![contacts[0]]));
        eprintln!("{name}: background-cancelled owned collision RMS {rms}");
        assert!(rms > 0.00001, "{name}: the actual physical callback must reach authored SFX output");
    }
}
