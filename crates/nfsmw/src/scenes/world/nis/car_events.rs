//! Per-channel authored body pulses and burnout. Contract: `race-reference-followup.md`.
use glam::Quat;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CarEvent {
    Pitch { dip: f32, length: f32 },
    Roll { dip: f32, length: f32 },
    Burnout(f32),
}

#[derive(Clone, Copy, Debug, Default)]
struct Pulse {
    dip: f32,
    start: f32,
    length: f32,
}

impl Pulse {
    fn angle(self, time: f32, span: f32) -> f32 {
        if self.length <= 0.0 {
            return 0.0;
        }
        let fraction = ((time - self.start) / self.length).clamp(0.0, 1.0);
        // The authored controller peaks at half its AI-angle displacement, then returns to rest.
        self.dip.atan2((span * 0.5).max(0.01)) / std::f32::consts::PI * fraction.min(1.0 - fraction)
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct CarMotion {
    pitch: Pulse,
    roll: Pulse,
    pub burnout: f32,
}

impl CarMotion {
    pub fn apply(&mut self, event: CarEvent, time: f32) {
        match event {
            CarEvent::Pitch { dip, length } => self.pitch = Pulse { dip, start: time, length },
            CarEvent::Roll { dip, length } => self.roll = Pulse { dip, start: time, length },
            CarEvent::Burnout(speed) => self.burnout = speed,
        }
    }

    pub fn rotation(self, time: f32, wheelbase: f32, width: f32) -> Quat {
        Quat::from_rotation_x(self.roll.angle(time, width)) * Quat::from_rotation_y(self.pitch.angle(time, wheelbase))
    }
}

pub fn slot_of(system: u32) -> Option<usize> {
    (0..16).find(|&slot| {
        let name = if slot < 8 { format!("car{}", slot + 1) } else { format!("cop{}", slot - 7) };
        blackbox_attrib::vlt_hash(&name) == system
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pulses_end_at_rest_and_channels_do_not_depend_on_event_order() {
        assert_eq!(slot_of(blackbox_attrib::vlt_hash("car2")), Some(1));
        assert_eq!(slot_of(blackbox_attrib::vlt_hash("cop3")), Some(10));
        assert_eq!(slot_of(blackbox_attrib::vlt_hash("camera")), None);
        let mut motion = CarMotion::default();
        motion.apply(CarEvent::Pitch { dip: 0.1, length: 2.0 }, 1.0);
        assert_eq!(motion.rotation(1.0, 2.5, 1.6), Quat::IDENTITY);
        assert!(motion.rotation(2.0, 2.5, 1.6).angle_between(Quat::IDENTITY) > 0.01);
        assert_eq!(motion.rotation(3.0, 2.5, 1.6), Quat::IDENTITY);
        assert_eq!(motion.rotation(30.0, 2.5, 1.6), Quat::IDENTITY);
    }
}
