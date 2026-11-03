//! PC world-effect proximity and crossing state, independent of rendering.
use glam::Vec3;
use nfsmw_data::world::emitter_placements::Trigger;

pub(super) fn visible(at: Vec3, far: f32, cosine: f32, views: &[(Vec3, Vec3)]) -> bool {
    views.iter().any(|(camera, forward)| {
        let offset = at - camera;
        let distance = offset.length();
        distance < far && distance > 0.0 && offset.dot(*forward) / distance > cosine
    })
}

#[derive(Default)]
enum Phase {
    #[default]
    Armed,
    Active,
    Cooling(f64),
}

#[derive(Default)]
pub(super) struct Crossing {
    phase: Phase,
    inside: bool,
}

impl Crossing {
    pub fn enter(&mut self, spec: &Trigger, actor: Vec3, random: impl FnOnce() -> u16) -> bool {
        if !matches!(self.phase, Phase::Armed) {
            return false;
        }
        let inside = actor.distance_squared(Vec3::from_array(spec.position)) < spec.radius.powi(2);
        let entered = inside && !self.inside;
        self.inside = inside;
        if !entered || spec.probability == 0 {
            return false;
        }
        if spec.probability != u16::MAX && random() <= spec.probability {
            return false;
        }
        self.phase = Phase::Active;
        true
    }

    pub fn update(&mut self, spec: &Trigger, alive: bool, now: f64) {
        match self.phase {
            Phase::Active if !alive => self.phase = Phase::Cooling(now),
            Phase::Cooling(ended) if now - ended > f64::from(spec.reset_time) => self.phase = Phase::Armed,
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn trigger(probability: u16) -> Trigger {
        Trigger { radius: 2.0, position: [0.0; 3], reset_time: 3.0, probability }
    }

    #[test]
    fn crossing_waits_for_group_death_cooldown_and_new_entry() {
        let spec = trigger(u16::MAX);
        let mut state = Crossing::default();
        assert!(!state.enter(&spec, Vec3::X * 2.0, || 0));
        assert!(state.enter(&spec, Vec3::ZERO, || 0));
        state.update(&spec, true, 100.0);
        assert!(!state.enter(&spec, Vec3::ZERO, || 0));
        state.update(&spec, false, 101.0);
        state.update(&spec, false, 104.0);
        assert!(matches!(state.phase, Phase::Cooling(_)));
        state.update(&spec, false, 104.01);
        assert!(!state.enter(&spec, Vec3::ZERO, || 0));
        assert!(!state.enter(&spec, Vec3::X * 2.0, || 0));
        assert!(state.enter(&spec, Vec3::ZERO, || 0));
    }

    #[test]
    fn failed_roll_does_not_repeat_until_exit() {
        let spec = trigger(100);
        let mut state = Crossing::default();
        assert!(!state.enter(&spec, Vec3::ZERO, || 100));
        assert!(!state.enter(&spec, Vec3::ZERO, || panic!("no reroll")));
        state.enter(&spec, Vec3::X * 3.0, || 0);
        assert!(state.enter(&spec, Vec3::ZERO, || 101));
        assert!(!Crossing::default().enter(&trigger(0), Vec3::ZERO, || panic!("disabled")));
    }

    #[test]
    fn admission_and_spawn_cones_differ_and_second_view_counts() {
        let views = [(Vec3::ZERO, Vec3::Y)];
        let at = Vec3::new(0.8, 0.6, 0.0);
        assert!(visible(at, 2.0, 0.55, &views));
        assert!(!visible(at, 2.0, 0.7, &views));
        assert!(!visible(Vec3::Y * 2.0, 2.0, 0.55, &views));
        assert!(!visible(-Vec3::Y, 2.0, 0.55, &views));
        assert!(visible(-Vec3::Y, 2.0, 0.55, &[(Vec3::ZERO, Vec3::Y), (Vec3::ZERO, -Vec3::Y)]));
    }
}
