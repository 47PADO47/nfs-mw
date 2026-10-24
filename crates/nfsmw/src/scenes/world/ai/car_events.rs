//! Apply each physical car's queued prop events to the shared world.

impl super::TrafficWorld {
    pub fn update_events(
        &mut self,
        props: &mut super::super::props::PropWorld,
        data: &nfsmw_data::car::physics::PhysicsData,
        view: glam::Vec3,
    ) {
        for car in &mut self.cars {
            car.update_events(props, data, view);
        }
    }
}
