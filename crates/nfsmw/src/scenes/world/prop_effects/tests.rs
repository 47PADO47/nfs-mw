use super::*;
use std::rc::Rc;

#[test]
fn wrapper_intensity_and_owner_velocity_are_separate_from_emitter_inheritance() {
    let mut fx = bank();
    let set = fx.sets.get_mut(&1).unwrap().as_mut().unwrap();
    set.parameters.quadratic = [0.5, 0.0, 0.0, 0.0];
    set.parameters.inherit_velocity = 0.5;
    set.templates[0].1.inherit = 1.0;
    set.templates[0].1.rate = 120.0;
    let owner = Rc::new(());
    let mut input = source(&owner, 0);
    input.effect.intensity = 0.0;
    input.frame.inherit_velocity = Vec3::X * 10.0;
    fx.start(input);
    fx.step(&super::super::props::PropWorld::default(), 1.0 / 60.0);
    let sprites: Vec<_> = fx.live[0].emitters[0].1.sprites().collect();
    assert_eq!(sprites.len(), 1, "zero input magnitude still activates the wrapper's constant term");
    assert!(
        (sprites[0].position.x - 5.0 / 60.0).abs() < 1e-6,
        "wrapper inheritance scales owner speed, independently from the half-intensity spawn rate"
    );
}

#[test]
fn tracking_moves_new_emissions_but_untracked_effects_keep_their_world_frame() {
    let mut fx = bank();
    let owner = Rc::new(());
    let mut moving = source(&owner, 1);
    moving.effect.tracking = true;
    moving.effect.intensity = 0.5;
    fx.start(moving);
    fx.start(source(&owner, 2));
    let frame = Frame {
        to_world: glam::Mat4::from_translation(Vec3::X * 20.0),
        inherit_velocity: Vec3::Y * 3.0,
        intensity: 1.0,
    };
    for live in &mut fx.live {
        live.refresh_frame(Some(frame));
    }
    assert_eq!(fx.live[0].source.frame.to_world, frame.to_world);
    assert_eq!(fx.live[0].source.frame.inherit_velocity, frame.inherit_velocity);
    assert_eq!(fx.live[0].source.frame.intensity, 0.5);
    assert_eq!(fx.live[1].source.frame.to_world, glam::Mat4::IDENTITY);
    for live in &mut fx.live {
        live.refresh_frame(None);
    }
    assert!(fx.live.iter().all(|live| live.stopping));
}

fn bank() -> ObjectFx {
    let mut fx = ObjectFx::default();
    fx.sets.insert(
        1,
        Some(Set {
            parameters: Default::default(),
            templates: vec![(
                1,
                EmitterSpec { rate: 60.0, life: 0.25, on_cycle: 0.1, size: [0.5; 4], ..Default::default() },
            )],
        }),
    );
    fx
}

fn source(owner: &Rc<()>, instance: u32) -> Source {
    Source {
        id: 7,
        owner: Rc::downgrade(owner),
        frame: Frame::default(),
        effect: ObjectEffect {
            name_tag: 0,
            effect: 1,
            position: 0,
            intensity: 1.0,
            instance,
            tracking: false,
            allow_hidden_spawn: false,
        },
    }
}

#[test]
fn world_burst_outlives_its_source_then_drains() {
    let mut fx = bank();
    let owner = Rc::new(());
    fx.start(source(&owner, 0));
    drop(owner);
    let props = super::super::props::PropWorld::default();
    fx.step(&props, 1.0 / 60.0);
    assert!(!fx.live[0].emitters[0].1.is_empty());
    for _ in 0..60 {
        fx.step(&props, 1.0 / 60.0);
    }
    assert!(fx.live.is_empty());
}

#[test]
fn stop_commands_are_scoped_to_source_owner_and_instance() {
    let mut fx = bank();
    let owner = Rc::new(());
    let other = Rc::new(());
    fx.start(source(&owner, 1));
    fx.start(source(&owner, 2));
    fx.start(source(&other, 1));
    fx.start(source(&owner, 0));
    fx.request([Command::Stop { source: 7, owner: Rc::downgrade(&owner), instance: Some(1) }]);
    fx.process_pending();
    assert_eq!(fx.live.iter().map(|i| i.stopping).collect::<Vec<_>>(), [true, false, false, false]);
    fx.request([Command::Stop { source: 7, owner: Rc::downgrade(&owner), instance: None }]);
    fx.process_pending();
    assert_eq!(fx.live.iter().map(|i| i.stopping).collect::<Vec<_>>(), [true, true, false, false]);
}

#[test]
fn replacing_the_same_key_does_not_duplicate_it_and_missing_sources_stop() {
    let mut fx = bank();
    let owner = Rc::new(());
    fx.start(source(&owner, 1));
    fx.start(source(&owner, 1));
    assert_eq!(fx.started, 1);
    fx.step(&super::super::props::PropWorld::default(), 1.0 / 60.0);
    assert!(fx.live.is_empty(), "a removed source stops even an untracked keyed effect");
}

#[test]
fn instance_and_queue_budgets_bound_repeated_requests() {
    let mut fx = bank();
    let owner = Rc::new(());
    for _ in 0..1000 {
        fx.request([Command::Play(source(&owner, 0))]);
    }
    assert_eq!(fx.pending.len(), MAX_PENDING);
    fx.process_pending();
    assert_eq!(fx.live.len(), MAX_INSTANCES);
    assert!(fx.rejected > 0);
}
