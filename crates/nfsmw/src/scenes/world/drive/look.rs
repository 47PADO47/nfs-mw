//! Look actions are frame deltas; preserve them across frames with no physics tick.

#[derive(Default)]
pub(super) struct PendingLook {
    x: f32,
    y: f32,
}

impl PendingLook {
    pub fn note(&mut self, (x, y): (f32, f32)) {
        self.x += x;
        self.y += y;
    }

    /// Spread the accumulated motion over this frame's ticks; never repeat or discard it.
    pub fn take(&mut self, steps: u32) -> (f32, f32) {
        if steps == 0 {
            return (0.0, 0.0);
        }
        let waiting = std::mem::take(self);
        (waiting.x / steps as f32, waiting.y / steps as f32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenes::world::drive::FixedClock;

    #[test]
    fn mouse_motion_waits_for_a_tick_and_is_consumed_once() {
        let mut look = PendingLook::default();
        look.note((4.0, -2.0));
        assert_eq!(look.take(0), (0.0, 0.0));
        look.note((6.0, 0.0));
        assert_eq!(look.take(2), (5.0, -1.0));
        assert_eq!(look.take(1), (0.0, 0.0));
    }

    #[test]
    fn stick_motion_is_not_multiplied_or_lost_at_different_frame_rates() {
        for fps in [5, 30, 60, 120, 144, 240] {
            let mut clock = FixedClock::default();
            let mut look = PendingLook::default();
            let mut total = (0.0, 0.0);
            for _ in 0..fps {
                let dt = 1.0 / fps as f32;
                look.note((700.0 * dt, -1400.0 * dt));
                let steps = clock.advance(dt);
                let delta = look.take(steps);
                total.0 += delta.0 * steps as f32;
                total.1 += delta.1 * steps as f32;
            }
            assert!((total.0 - 700.0).abs() < 0.002, "{fps} FPS: {total:?}");
            assert!((total.1 + 1400.0).abs() < 0.004, "{fps} FPS: {total:?}");
        }
    }
}
