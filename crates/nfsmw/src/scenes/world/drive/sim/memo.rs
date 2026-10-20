//! What the last physics step did to the car, for the police radio (`SoundAI::OnCollision` reads the same facts from the
//! original's collision callback): the pedals, and the one thing the car reacted against in the static world or among the
//! props. Kept so the traffic world, which steps after the car, can tell the radio without owning the car's contacts.

use glam::Vec3;

use super::{CarSim, ContactKind, Impact, InputState};

/// The thing the car reacted against in one step.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SurfaceHit {
    pub kind: ContactKind,
    /// The speed the car lost to the reaction, metres per second.
    pub delta_v: f32,
    /// How fast the car met the surface, metres per second.
    pub closing: f32,
}

#[derive(Debug, Clone, Default)]
pub struct StepMemo {
    pub brake: f32,
    pub handbrake: f32,
    /// The speed before the step's motion and after its contacts, metres per second.
    pub speed_before: f32,
    pub speed_after: f32,
    pub hit: Option<SurfaceHit>,
    /// The props the car touched this step (ids in the prop world).
    pub props: Vec<u32>,
}

impl StepMemo {
    /// Notes a step: `before` is the car's velocity when the step began, `after` when its contacts were resolved and
    /// `mass` the car's, kilograms.
    pub fn record(&mut self, input: &InputState, impact: &Impact, before: Vec3, after: Vec3, mass: f32) {
        self.brake = input.brake;
        self.handbrake = input.handbrake;
        self.speed_before = before.length();
        self.speed_after = after.length();
        self.props.clear();
        self.props.extend(impact.visuals.iter().filter_map(|v| v.prop));
        self.props.extend(impact.knocked.iter().map(|&(id, _)| id));
        self.hit = None;
        if impact.impulse <= 0.0 || mass <= 0.0 {
            return;
        }
        let closing = impact.visuals.iter().find(|v| v.impulse_delta_v > 0.0);
        self.hit = Some(SurfaceHit {
            kind: closing.map_or(ContactKind::PropRigid, |v| v.kind),
            delta_v: impact.impulse / mass,
            closing: closing.map_or(self.speed_before, |v| v.velocity.length()),
        });
    }
}

impl CarSim {
    /// What the last step did to the car: the pedals, the speed, the one surface it reacted against and the props it
    /// touched. The police radio reads it after the step.
    pub fn step_memo(&self) -> &StepMemo {
        &self.memo
    }

    /// Pretends the last step reacted against `hit`.
    #[cfg(test)]
    pub fn force_step_hit(&mut self, hit: SurfaceHit) {
        self.memo.hit = Some(hit);
    }

    /// The surface the last step reacted against, taken so that it is told once.
    pub fn take_step_hit(&mut self) -> Option<SurfaceHit> {
        self.memo.hit.take()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenes::world::drive::walls::VisualContact;

    fn visual(kind: ContactKind, delta_v: f32, velocity: Vec3, prop: Option<u32>) -> VisualContact {
        VisualContact {
            point: Vec3::ZERO,
            normal: Vec3::X,
            velocity,
            impulse_delta_v: delta_v,
            surface: None,
            kind,
            info: None,
            prop,
        }
    }

    #[test]
    fn a_wall_the_car_reacted_to_is_remembered_with_its_closing_speed() {
        let mut memo = StepMemo::default();
        let mut impact = Impact { impulse: 20_000.0, ..Impact::default() };
        impact.visuals.push(visual(ContactKind::Barrier, 10.0, Vec3::new(0.0, 0.0, 30.0), None));
        let input = InputState { brake: 1.0, ..InputState::default() };
        memo.record(&input, &impact, Vec3::new(0.0, 0.0, 31.0), Vec3::new(0.0, 0.0, 10.0), 2000.0);
        let hit = memo.hit.expect("a hit");
        assert_eq!(hit.kind, ContactKind::Barrier);
        assert_eq!(hit.delta_v, 10.0);
        assert_eq!(hit.closing, 30.0);
        assert_eq!((memo.speed_before, memo.speed_after, memo.brake), (31.0, 10.0, 1.0));
    }

    #[test]
    fn a_quiet_step_remembers_no_hit_and_the_props_it_touched() {
        let mut memo = StepMemo::default();
        let mut impact = Impact::default();
        impact.visuals.push(visual(ContactKind::PropRigid, 0.0, Vec3::ZERO, Some(7)));
        impact.knocked.push((9, 20.0));
        memo.record(&InputState::default(), &impact, Vec3::Z, Vec3::Z, 1500.0);
        assert!(memo.hit.is_none());
        assert_eq!(memo.props, vec![7, 9]);
    }

    #[test]
    fn an_impulse_with_no_reacting_contact_is_a_prop_hit_at_the_cars_own_speed() {
        let mut memo = StepMemo::default();
        let impact = Impact { impulse: 3_000.0, ..Impact::default() };
        memo.record(&InputState::default(), &impact, Vec3::new(0.0, 0.0, 12.0), Vec3::new(0.0, 0.0, 9.0), 1500.0);
        let hit = memo.hit.expect("a hit");
        assert_eq!((hit.kind, hit.closing, hit.delta_v), (ContactKind::PropRigid, 12.0, 2.0));
    }
}
