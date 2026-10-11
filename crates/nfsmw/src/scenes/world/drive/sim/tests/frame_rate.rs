//! The contact sequence and all body states must be invariant under render-time grouping.

use super::{DriveInput, FlatGround, Spawn, Vec3, sim};
use crate::scenes::world::drive::FixedClock;

#[derive(Debug, PartialEq)]
struct Trace {
    steps: u32,
    states: Vec<u32>,
    hits: Vec<(u32, usize, usize, u32)>,
}

fn run(frame_times: impl IntoIterator<Item = f32>) -> Trace {
    let ground = FlatGround::new(5.0);
    let mut cars = [sim(), sim(), sim()];
    for (car, (x, heading, speed)) in
        cars.iter_mut().zip([(100.0, 0.0, 25.0), (112.0, 0.0, 0.0), (145.0, std::f32::consts::PI, 20.0)])
    {
        car.configure_ai();
        super::geometry::compound(car);
        assert!(car.place_moving(&ground, Spawn { position: Vec3::new(x, 50.0, 5.0), heading }, speed));
    }
    cars[1].body_mut().set_mass(2500.0);
    cars[0].collision_role.player = true;
    super::reactions::install(&mut cars[1]);
    cars[2].collision_racer = true;
    let mut clock = FixedClock::default();
    let mut trace = Trace { steps: 0, states: Vec::new(), hits: Vec::new() };
    for dt in frame_times {
        for _ in 0..clock.advance(dt) {
            for car in &mut cars {
                car.step(&DriveInput::default(), &ground, None);
            }
            for i in 0..cars.len() {
                let (head, tail) = cars.split_at_mut(i + 1);
                for (offset, other) in tail.iter_mut().enumerate() {
                    if let Some(hit) = head[i].collide_with(other) {
                        trace.hits.push((trace.steps, i, i + 1 + offset, hit.impulse.to_bits()));
                    }
                }
            }
            for car in &cars {
                let body = car.body();
                assert!(car.is_finite());
                trace.states.extend(
                    body.position
                        .to_array()
                        .into_iter()
                        .chain(body.orientation.to_array())
                        .chain(body.linear_velocity.to_array())
                        .chain(body.angular_velocity.to_array())
                        .map(f32::to_bits),
                );
            }
            trace.steps += 1;
        }
    }
    trace
}

#[test]
fn identical_contacts_and_states_at_low_high_and_uneven_frame_rates() {
    let seconds = 6;
    let reference = run(std::iter::repeat_n(1.0 / 60.0, 60 * seconds));
    assert_eq!(reference.steps, 360);
    assert!(!reference.hits.is_empty(), "the scenario must exercise contacts");
    for fps in [5, 10, 30, 120, 144, 240] {
        let trace = run(std::iter::repeat_n(1.0 / fps as f32, fps * seconds));
        assert_eq!(trace.steps, reference.steps, "tick count at {fps} FPS");
        assert_eq!(trace.hits, reference.hits, "contact sequence at {fps} FPS");
        assert_eq!(trace.states, reference.states, "body states at {fps} FPS");
    }
    // Include a 300 ms stall: all 18 owed steps must run, with a contact pass after each one.
    let cycle: [f32; 5] = [1.0 / 144.0, 1.0 / 30.0, 0.3, 1.0 / 240.0, 1.0 / 65.0];
    let mut remaining = seconds as f64;
    let mut frames = Vec::new();
    for dt in cycle.into_iter().cycle() {
        if remaining <= 1e-10 {
            break;
        }
        let elapsed = (dt as f64).min(remaining) as f32;
        frames.push(elapsed);
        remaining -= f64::from(elapsed);
    }
    let uneven = run(frames);
    assert_eq!(uneven.steps, reference.steps);
    assert_eq!(uneven.hits, reference.hits, "contacts after uneven frames and stalls");
    assert_eq!(uneven.states, reference.states, "body states after uneven frames and stalls");
}
