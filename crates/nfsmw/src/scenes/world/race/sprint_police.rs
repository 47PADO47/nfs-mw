//! Police trigger crossings in the prologue sprint (story-flow.md section 2.4).

use glam::Vec3;

use super::Stage;
use super::scripted_chase::{Zone, cop_points, zone_named};
use crate::scenes::world::{WorldScene, space};

pub(super) const EVENT: &str = "16.2.3";
const SPAWN_TRIGGER: &str = "cop_spawn_trigger";
const BAIL_TRIGGER: &str = "bail_pursuit";

/// The patrol trigger's per-run crossing and pending request.
#[derive(Debug)]
pub(in crate::scenes::world) struct SprintPolice {
    zones: [Option<Zone>; 2],
    last_point: Option<Vec3>,
    inside: [bool; 2],
    enabled: bool,
    pending: bool,
}

impl SprintPolice {
    fn new(zones: [Option<Zone>; 2]) -> Self {
        Self { zones, last_point: None, inside: [false; 2], enabled: false, pending: false }
    }

    fn step(&mut self, point: Vec3) -> bool {
        let previous = self.last_point.replace(point).unwrap_or(point);
        let [spawn_entered, bail_entered] = std::array::from_fn(|index| {
            let Some(zone) = &self.zones[index] else { return false };
            let was_inside = std::mem::replace(&mut self.inside[index], zone.contains(point));
            !was_inside && zone.touched(previous, point)
        });
        if spawn_entered {
            self.enabled = true;
            self.pending = true;
        }
        if bail_entered {
            self.pending = false;
        }
        bail_entered
    }

    pub(super) fn enabled(&self) -> bool {
        self.enabled
    }
}

impl WorldScene {
    pub(in crate::scenes::world) fn update_sprint_police(&mut self) {
        let Some(session) = self.race.as_ref().filter(|session| {
            session.options.career
                && session.event == EVENT
                && session.stage == Stage::Running
                && session.engine.as_ref().is_some_and(|engine| engine.phase() == blackbox_race::Phase::Racing)
        }) else {
            return;
        };
        let Some(drive) = self.players.lead().filter(|drive| drive.has_sim() && drive.waiting_for_road().is_none())
        else {
            return;
        };
        let point = Vec3::from(space::to_physics(drive.position()));
        if self.career.sprint_police.is_none() {
            self.career.sprint_police = self.gameplay.as_ref().map(|data| {
                SprintPolice::new([SPAWN_TRIGGER, BAIL_TRIGGER].map(|name| zone_named(data, &session.name, name)))
            });
        }
        if self.career.sprint_police.as_mut().is_some_and(|police| police.step(point)) {
            if let Some(traffic) = self.traffic.as_mut() {
                traffic.suppress_new_cops(true);
            }
            crate::parity::career_trace::record("race_police_suppressed", serde_json::json!({"event": session.event}));
        }
    }

    pub(in crate::scenes::world) fn take_sprint_patrols(&mut self) -> Vec<(Vec3, Vec3)> {
        let (Some(police), Some(session), Some(data)) =
            (self.career.sprint_police.as_mut(), self.race.as_ref(), self.gameplay.as_ref())
        else {
            return Vec::new();
        };
        if session.event != EVENT || session.stage != Stage::Running || !std::mem::take(&mut police.pending) {
            return Vec::new();
        }
        let points = cop_points(data, &session.name);
        crate::parity::career_trace::record(
            "race_patrol_request",
            serde_json::json!({"event": session.event, "count": points.len(), "model": "copmidsize"}),
        );
        points
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nfsmw_data::gameplay::PlaneTrigger;

    fn plane(name: &str, z: f32) -> Zone {
        Zone::Plane(PlaneTrigger {
            key: 1,
            name: name.to_owned(),
            position: Vec3::Z * z,
            heading_deg: 0.0,
            width: 20.0,
            resets_player: false,
        })
    }

    #[test]
    fn crossing_the_plane_requests_patrols_until_consumed_and_can_fire_again() {
        let mut run = SprintPolice::new([Some(plane(SPAWN_TRIGGER, 0.0)), None]);
        run.step(Vec3::NEG_Z);
        assert!(!run.enabled());
        assert!(!run.pending);
        run.step(Vec3::Z);
        assert!(run.enabled());
        assert!(std::mem::take(&mut run.pending));
        run.step(Vec3::Z * 2.0);
        assert!(!run.pending);
        run.step(Vec3::ZERO);
        assert!(std::mem::take(&mut run.pending));
        run.step(Vec3::ZERO);
        assert!(!run.pending);
    }

    #[test]
    fn crossing_bail_suppresses_new_police_and_drops_an_unsubmitted_spawn() {
        let mut run = SprintPolice::new([Some(plane(SPAWN_TRIGGER, 0.0)), Some(plane(BAIL_TRIGGER, 4.0))]);
        assert!(!run.step(Vec3::NEG_Z));
        assert!(!run.step(Vec3::Z));
        assert!(run.pending);
        assert!(run.step(Vec3::Z * 4.0));
        assert!(run.enabled(), "active cops are still permitted");
        assert!(!run.pending);
        assert!(!run.step(Vec3::Z * 4.0));
    }
}
