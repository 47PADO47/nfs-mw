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
