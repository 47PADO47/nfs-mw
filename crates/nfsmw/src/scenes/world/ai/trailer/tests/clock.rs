use super::*;
use crate::scenes::world::drive::FixedClock;
use blackbox_vehicle::rigid_body::{ContactParams, PlaneContact};

fn trace(fps: u32) -> Vec<Vec<f32>> {
    let ground = FlatGround::new(5.0);
    let (mut tractor, mut trailer, mut coupling) = rig(&ground, 12.0);
    let mut clock = FixedClock::default();
    let mut out = Vec::new();
    let mut hit_count = 0;
    for _ in 0..fps * 4 {
        for _ in 0..clock.advance(1.0 / fps as f32) {
            let tick = out.len();
            let input = DriveInput {
                throttle: 0.6,
                steer: match tick < 180 {
                    true => 0.3,
                    false => 0.0,
                },
                brake: match tick >= 180 {
                    true => 1.0,
                    false => 0.0,
                },
                ..DriveInput::default()
            };
            tractor.step(&input, &ground, None);
            trailer.step(&DriveInput { brake: input.brake, ..DriveInput::default() }, &ground, None);
            if tick == 120 {
                let body = trailer.body_mut();
                let contact = PlaneContact {
                    point: body.world_cog() + body.rotation().x_axis * 0.7,
                    normal: -body.linear_velocity.normalize(),
                    friction_static: 0.2,
                    friction_kinetic: 0.1,
                };
                let hit = body.react_plane(&contact, &ContactParams { restitution: 0.1, inertia_scale: Vec3::ONE });
                assert!(hit.is_some_and(|hit| hit.impulse > 0.0));
                hit_count += 1;
            }
            coupling.hold(&mut tractor, &mut trailer);
            assert!(tractor.is_finite() && trailer.is_finite());
            if coupling.is_hitched() {
                assert!(coupling.gap(&tractor, &trailer) < 0.0001);
            }
            let mut sample = Vec::new();
            for body in [tractor.body(), trailer.body()] {
                sample.extend(body.position.to_array());
                sample.extend(body.orientation.to_array());
                sample.extend(body.linear_velocity.to_array());
                sample.extend(body.angular_velocity.to_array());
            }
            out.push(sample);
        }
    }
    assert_eq!(hit_count, 1);
    assert_eq!(out.len(), 240);
    out
}

#[test]
fn turning_braking_and_collision_have_identical_complete_traces_at_every_frame_rate() {
    let expected = trace(60);
    for fps in [5, 15, 30, 120, 144, 240] {
        assert_eq!(trace(fps), expected, "{fps} FPS");
    }
}

#[test]
fn articulation_velocity_reaches_the_same_ticks_driving_response() {
    fn response(early: bool) -> (Vec3, Vec3) {
        let ground = FlatGround::new(5.0);
        let (mut tractor, mut trailer, coupling) = rig(&ground, 12.0);
        trailer.body_mut().linear_velocity = Vec3::ZERO;
        let input = DriveInput { throttle: 0.8, steer: 0.3, ..Default::default() };
        let a = tractor.begin_step_with_delta(STEP, &input, false, None);
        let b = trailer.begin_step_with_delta(STEP, &DriveInput::default(), false, None);
        tractor.begin_body(&a, None);
        trailer.begin_body(&b, None);
        tractor.finish_body(&a, &ground, None);
        trailer.finish_body(&b, &ground, None);
        let before = tractor.body().linear_velocity;
        if early {
            coupling.resolve_with_delta(STEP, &mut tractor, &mut trailer);
        }
        let response_velocity = tractor.body().linear_velocity;
        tractor.integration().run(&a, &ground, None);
        trailer.integration().run(&b, &ground, None);
        let force = tractor.body().pending_force();
        if !early {
            coupling.resolve_with_delta(STEP, &mut tractor, &mut trailer);
        }
        assert!((tractor.body().linear_velocity - before).length() > 0.1, "the hitch must transfer velocity");
        (response_velocity, force)
    }
    let early = response(true);
    let late = response(false);
    assert!((early.0 - late.0).length() > 0.1, "the response must receive the articulation impulse");
    assert!((early.1 - late.1).length() > 0.1, "postponing the hitch must change this tick's generated forces");
}
