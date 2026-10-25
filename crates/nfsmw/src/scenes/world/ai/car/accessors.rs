//! Small state accessors and setters of [`AiCar`], kept out of `car.rs` to keep that file short.

use super::*;

impl AiCar {
    /// Simulation time covered by the next staggered pursuit decision.
    pub(in crate::scenes::world::ai) fn next_think_delta(&self, dt: f32) -> f32 {
        match self.has_thought {
            true => self.think_elapsed + dt,
            false => super::cop::THINK_STEPS as f32 * dt,
        }
    }

    /// Makes this car a roadblock car: it stays where it was placed with the brakes on.
    pub fn set_roadblock(&mut self) {
        self.roadblock = true;
        self.breaker_stop = true;
        self.sim.collision_role.cop = true;
        self.sim.collision_goal = Some(super::ROADBLOCK_GOAL);
        if self.damage.is_none() {
            self.damage = Some(CopDamage::for_model(&self.name, self.sim.events.as_ref().map(|e| e.spec())));
        }
    }

    /// Whether this car is part of a roadblock (and so holds its place).
    pub fn is_roadblock(&self) -> bool {
        self.roadblock
    }

    /// Puts this roadblock car in the standing roadblock `group`.
    pub fn set_roadblock_group(&mut self, group: u32) {
        self.roadblock_group = group;
    }

    /// The standing roadblock this car belongs to, if any.
    pub fn roadblock_group(&self) -> Option<u32> {
        (self.roadblock && self.roadblock_group != 0).then_some(self.roadblock_group)
    }

    /// The car's identity for the roadblock and the pursuit: its entity id in the world, stable for the car's life and
    /// independent of render-side work such as model uploads (`docs/specs/multiplayer/determinism.md`, D5).
    pub fn key(&self) -> u32 {
        self.entity
    }

    /// Gives the car its entity id as it joins the world (`TrafficWorld::admit`).
    pub(in crate::scenes::world::ai) fn set_entity(&mut self, entity: u32) {
        self.entity = entity;
    }

    /// A roadblock car joins the chase as an ordinary pursuit cop, as the cheat rule recruits it (`AIPursuit` takes the
    /// nearest car of the roadblock into its own list). It keeps its place, speed and damage and is driven by the
    /// pursuit from the next step. Returns whether it was a roadblock car that could join.
    pub(in crate::scenes::world::ai) fn recruit_to_pursuit(
        &mut self,
        car: super::cop::CopCar,
        player: blackbox_vehicle::performance::Performance,
        net: &blackbox_roads::RoadNetwork,
        index: &blackbox_roads::SegmentIndex,
    ) -> bool {
        if !self.roadblock || self.is_wrecked() {
            return false;
        }
        self.roadblock = false;
        self.roadblock_group = 0;
        self.breaker_stop = false;
        self.patrol = true;
        let joined = self.join_pursuit(car, player, net, index);
        self.patrol = false;
        joined
    }

    pub fn set_breaker_stop(&mut self, stop: bool) {
        self.breaker_stop = stop;
    }

    pub fn pose(&self, alpha: f32) -> CarPose {
        self.previous.lerp(&self.current, alpha)
    }

    /// The pose the car is drawn at: the NIS scene's pose while a scene drives it, otherwise the physics pose at `alpha`.
    pub fn render_pose(&self, alpha: f32) -> CarPose {
        self.nis_pose.unwrap_or_else(|| self.pose(alpha))
    }

    /// Sets the pose a NIS scene gives the car, or clears it with `None`.
    pub fn set_nis_pose(&mut self, pose: Option<CarPose>) {
        self.nis_pose = pose;
    }

    /// Collision separation belongs to this tick's displayed pose too.
    pub fn sync_contacts(&mut self) {
        if let Some(trailer) = self.trailer.as_mut() {
            trailer.sync_contacts(&mut self.sim);
        }
        self.sim.finish_retirement_contacts();
        self.current = self.sim.pose();
    }

    pub fn is_finite(&self) -> bool {
        self.sim.is_finite()
    }

    /// Seconds the car has had no wheel on the ground (it fell off the map, or flew).
    pub fn airborne_time(&self) -> f32 {
        self.airborne
    }

    /// Seconds the car has been standing still.
    pub fn idle_time(&self) -> f32 {
        self.idle
    }

    /// Speed along the car, m/s.
    pub fn speed(&self) -> f32 {
        self.sim.telemetry().speed_mps
    }

    /// A line about what the car is doing, for the `traffic status` command.
    pub fn debug(&self) -> String {
        let damage = self.damage_status();
        format!(
            "{} segment {} lane {} t {:.2}{}{}{}, wants {:.1} m/s{damage}",
            if self.is_cop() {
                "cop"
            } else if self.patrol {
                "patrol"
            } else {
                "traffic"
            },
            self.nav.segment,
            self.nav.lane,
            self.nav.t,
            if self.blocked { ", blocked" } else { "" },
            if self.waiting { ", waits for a light" } else { "" },
            if self.nav.dead_end { ", dead end" } else { "" },
            self.speed
        )
    }

    pub fn controls(&self) -> AiControls {
        self.controls
    }

    /// An identity of the car that holds for its life (the radio keys its cops by it).
    pub(in crate::scenes::world::ai) fn render_key(&self) -> u32 {
        self.render_id
    }

    /// A cop chasing the player.
    pub fn is_cop(&self) -> bool {
        matches!(self.role, Role::Cop(_)) && !self.is_wrecked()
    }

    /// A cop car cruising like traffic.
    pub fn is_patrol(&self) -> bool {
        self.patrol
    }

    /// Adds `dt` to the time the car has been out of view, or clears it when it is in view.
    pub fn note_view(&mut self, in_view: bool, dt: f32) {
        self.offscreen = match in_view {
            true => 0.0,
            false => self.offscreen + dt,
        };
    }

    pub fn offscreen_time(&self) -> f32 {
        self.offscreen
    }

    /// Seconds since the car was placed.
    pub fn age(&self) -> f32 {
        self.elapsed
    }

    /// Whether the patrol pass may recycle this cop now, given whether its spot is available to recycle
    /// (`AIVehicle::CanRespawn`). Remembers a visit to the area that is not.
    pub fn may_respawn(&mut self, spot_available: bool) -> bool {
        let age = self.age();
        self.respawn.can_respawn(spot_available, age)
    }

    /// Half the car's diagonal (metres): the radius of its bounding sphere.
    pub fn bounding_radius(&self) -> f32 {
        self.radius
    }

    /// The owner handle of the car's detached parts (`props::fragments`): they live while it does.
    pub fn part_owner(&self) -> Option<Weak<()>> {
        self.sim.events.as_ref().map(|events| Rc::downgrade(&events.owner))
    }

    /// Turns the rewrite's lane-change avoidance on or off for this car (`car/avoid.rs`).
    pub fn set_avoidance(&mut self, on: bool) {
        self.avoid.set_enabled(on);
    }

    /// The player car's speed (m/s) this physics step, read by the crash gate in `hits.rs`.
    pub fn set_player_speed(&mut self, speed: f32) {
        self.player_speed = speed;
    }
}

impl AiCar {
    /// Where the car is in physics space and which way it faces, for the spawner's spacing test.
    /// The rolling radius of each wheel, in the model's wheel order.
    pub fn wheel_radii(&self) -> [f32; 4] {
        self.sim.wheel_radii()
    }

    pub fn physics_position(&self) -> Vec3 {
        self.sim.driver_view().position
    }

    /// Which way the car faces, in physics space (unit length): the axis of its rear cone of sight.
    pub fn physics_forward(&self) -> Vec3 {
        let forward = self.sim.driver_view().forward;
        forward.try_normalize().unwrap_or(Vec3::Z)
    }
}
