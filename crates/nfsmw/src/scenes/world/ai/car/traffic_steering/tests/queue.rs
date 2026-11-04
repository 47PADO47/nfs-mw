//! Queue regression using synthetic VehicleSpec::example with Reference tire integration.
//! The player is an avoidable without collision response: attempted overlap must fail first.
//! These inputs do not establish complete original-game route or following-gap parity.

use super::*;
use blackbox_vehicle::tires::TireIntegration;

const WAIT_STEPS: u32 = 20 * 60;
const SETTLED_STEPS: u32 = 5 * 60;
const CLEAR_STEPS: u32 = 3 * 60;

struct QueueRig {
    car: AiCar,
    net: RoadNetwork,
    index: SegmentIndex,
    collision: CollisionWorld,
    surfaces: SurfaceTable,
    props: PropWorld,
    rng: SplitMix,
}

impl QueueRig {
    fn new() -> Self {
        let (car, net, collision, surfaces) = fixture_with_tires(false, 10.0, TireIntegration::Reference);
        let index = SegmentIndex::build(&net);
        Self { car, net, index, collision, surfaces, props: PropWorld::default(), rng: SplitMix(42) }
    }

    fn tick(&mut self, player: Option<Body>) {
        let bodies = [(self.car.body(), false)];
        let mut ctx = Ctx {
            net: &self.net,
            index: &self.index,
            rng: &mut self.rng,
            bodies: &bodies,
            me: 0,
            player,
            aims: &[],
            lights: None,
        };
        self.car.step(&mut ctx, &self.collision, &mut self.props, &self.surfaces);
    }

    fn finite_pose(&self) -> bool {
        let body = self.car.body();
        body.position.is_finite() && body.velocity.is_finite() && self.car.sim.pose().rotation.is_finite()
    }

    fn state(&self) -> String {
        let body = self.car.body();
        let facing = glam::Vec2::new(self.car.target.x - body.position.x, self.car.target.z - body.position.z)
            .try_normalize()
            .map_or(0.0, |toward| body.forward.dot(toward));
        format!(
            "position={:?} velocity={:?} wanted={} blocked={} facing={} gas={} brake={} steer={} handbrake={} target={:?}",
            body.position,
            body.velocity,
            self.car.speed,
            self.car.blocked,
            facing,
            self.car.controls.gas,
            self.car.controls.brake,
            self.car.controls.steer,
            self.car.controls.handbrake,
            self.car.target,
        )
    }
}

/// A positive result means separated on at least one rectangle axis; otherwise the bodies overlap.
fn planar_separation(me: &Body, player: &Body) -> f32 {
    let right = |forward: glam::Vec2| glam::Vec2::new(forward.y, -forward.x);
    let delta = glam::Vec2::new(player.position.x - me.position.x, player.position.z - me.position.z);
    let extent = |body: &Body, axis: glam::Vec2| {
        body.forward.dot(axis).abs() * body.half_length + right(body.forward).dot(axis).abs() * body.half_width
    };
    [me.forward, right(me.forward), player.forward, right(player.forward)]
        .into_iter()
        .map(|axis| delta.dot(axis).abs() - extent(me, axis) - extent(player, axis))
        .fold(f32::NEG_INFINITY, f32::max)
}

fn queue_case(distance: f32, lateral_offset: f32) {
    let mut rig = QueueRig::new();
    let mut player = rig.car.body();
    player.position += Vec3::new(lateral_offset, 0.0, distance);
    player.velocity = Vec3::ZERO;
    player.forward = -player.forward;
    assert!(planar_separation(&rig.car.body(), &player) > 0.0, "fixture starts without contact");
    let mut saw_braking = false;
    let mut reached_stop = false;
    let mut settled_position = rig.car.body().position;
    for tick in 1..=WAIT_STEPS {
        rig.tick(Some(player));
        let body = rig.car.body();
        assert!(rig.finite_pose(), "distance={distance}, lateral={lateral_offset}, tick={tick}: {}", rig.state());
        assert!(
            planar_separation(&body, &player) > 0.0,
            "attempted overlap: distance={distance}, lateral={lateral_offset}, tick={tick}: {}",
            rig.state()
        );
        saw_braking |= rig.car.controls.brake > 0.5;
        reached_stop |= body.velocity.length() < 0.25;
        if tick == WAIT_STEPS - SETTLED_STEPS {
            settled_position = body.position;
        }
        if tick > WAIT_STEPS - SETTLED_STEPS {
            assert!(
                body.velocity.length() < 0.25 && body.position.distance(settled_position) < 0.05,
                "queue did not stay stopped: distance={distance}, lateral={lateral_offset}, tick={tick}: {}",
                rig.state()
            );
        }
    }
    assert!(
        saw_braking && reached_stop,
        "no braking stop: distance={distance}, lateral={lateral_offset}: {}",
        rig.state()
    );
    let clear_start = rig.car.body().position;
    let mut saw_gas = false;
    for tick in 1..=CLEAR_STEPS {
        rig.tick(None);
        assert!(rig.finite_pose(), "clear road, tick={tick}: {}", rig.state());
        saw_gas |= rig.car.controls.gas > 0.5;
    }
    assert!(
        saw_gas && rig.car.body().position.distance(clear_start) > 0.5,
        "no restart after player removal: distance={distance}, lateral={lateral_offset}: {}",
        rig.state()
    );
}

#[test]
fn traffic_parity_reference_queue_holds_then_restarts_when_player_leaves() {
    for (distance, lateral) in [(14.0, 0.0), (14.0, 0.5), (30.0, 0.0), (30.0, 0.5)] {
        queue_case(distance, lateral);
    }
}
