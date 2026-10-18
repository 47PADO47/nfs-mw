//! Small state accessors and setters of [`AiCar`], kept out of `car.rs` to keep that file short.

use super::*;

impl AiCar {
    /// Makes this car a roadblock car: it stays where it was placed with the brakes on.
    pub fn set_roadblock(&mut self) {
        self.roadblock = true;
        self.breaker_stop = true;
        self.sim.collision_role.cop = true;
        self.damage.get_or_insert_with(|| CopDamage::for_model(&self.name));
    }

    /// Whether this car is part of a roadblock (and so holds its place).
    pub fn is_roadblock(&self) -> bool {
        self.roadblock
    }

    pub fn set_breaker_stop(&mut self, stop: bool) {
        self.breaker_stop = stop;
    }

    pub fn pose(&self, alpha: f32) -> CarPose {
        self.previous.lerp(&self.current, alpha)
    }

    /// Collision separation belongs to this tick's displayed pose too.
    pub fn sync_contacts(&mut self) {
        if let Some(trailer) = self.trailer.as_mut() {
            trailer.sync_contacts(&mut self.sim);
        }
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
        format!(
            "{} segment {} lane {} t {:.2}{}{}{}, wants {:.1} m/s",
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
