//! The car's physics for code that is not about the player, such as hits between cars.

use glam::{Vec2, Vec3};

use super::{CarSim, Drive};
use crate::scenes::world::space;

impl Drive {
    /// Current physical coordinates for police; visual interpolation and model pivots stay in rendering.
    fn sensing_pose(&self) -> (Vec3, Vec3) {
        if let Some(sim) = self.sim.as_ref() {
            let body = sim.driver_view();
            return (body.position, body.forward);
        }
        (
            Vec3::from_array(space::to_physics(self.current.position)),
            Vec3::from_array(space::to_physics(self.current.rotation * Vec3::X)),
        )
    }

    pub fn player_view(&self) -> crate::scenes::world::ai::PlayerView {
        let (position, forward) = self.sensing_pose();
        crate::scenes::world::ai::PlayerView {
            position: space::to_render(position.to_array()),
            forward: space::to_render(forward.to_array()),
            speed: self.speed(),
            nitrous: self.telemetry.nitrous_burn(),
            throttle: self.frame_input.throttle,
        }
    }

    pub(in crate::scenes::world) fn ai_body(&self) -> blackbox_roads::Body {
        let (position, forward) = self.sensing_pose();
        let velocity = self.sim.as_ref().map_or(forward * self.speed(), CarSim::velocity);
        let (half_width, half_length) = self.half_extents();
        blackbox_roads::Body {
            position,
            velocity,
            forward: Vec2::new(forward.x, forward.z).normalize_or_zero(),
            half_width,
            half_length,
        }
    }

    pub fn update_events(
        &mut self,
        props: &mut crate::scenes::world::props::PropWorld,
        data: &nfsmw_data::car::physics::PhysicsData,
        view: Vec3,
    ) {
        for id in self.pending_knocks.drain(..) {
            log::info!("knocked over prop {id} \"{}\" (hidden for a few seconds)", props.name(id).unwrap_or("?"));
            props.knock(id);
        }
        if let Some(sim) = self.sim.as_mut() {
            crate::scenes::world::cargo::update(sim, &self.rig, props, data, view);
        }
    }

    /// What the car can do flat out (measured by driving the model flat out), for cops that are matched to it.
    pub(in crate::scenes::world) fn performance(&self) -> blackbox_vehicle::performance::Performance {
        nfsmw_data::car::physics::measured::performance_of_spec(&self.physics.spec, self.physics.tire_integration)
    }

    /// The physics of the car, once it stands on a road.
    pub(in crate::scenes::world) fn sim(&self) -> Option<&CarSim> {
        self.sim.as_ref()
    }

    /// The physics of the car, once it stands on a road, to change.
    pub(in crate::scenes::world) fn sim_mut(&mut self) -> Option<&mut CarSim> {
        self.sim.as_mut()
    }
}
