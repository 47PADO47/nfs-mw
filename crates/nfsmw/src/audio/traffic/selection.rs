//! Pure producer gates, fixed-time horn fade and authored variant cycling.
use glam::Vec3;

#[derive(Default)]
pub(super) struct Variants {
    next: usize,
}

impl Variants {
    pub fn select(&mut self, kind: u32, speed: f32) -> Option<usize> {
        if !speed.is_finite() || speed <= 10.0 {
            return None;
        }
        let (base, blocks, width) = match kind {
            8 | 9 => (45, 2, 6),
            1 => (57, 3, 6),
            2 => (9, 3, 6),
            12 => (21, 1, 6),
            3 => (33, 2, 6),
            4 => (75, 3, 3),
            5 | 7 => (84, 2, 3),
            11 => (27, 2, 3),
            0 | 6 | 10 => (0, 3, 3),
            _ => return None,
        };
        let intensity = ((speed - 10.0) / 50.0).clamp(0.0, 0.99) * 127.0;
        let block = (intensity / 128.0 * blocks as f32) as usize;
        let id = base + block * width + self.next % width;
        self.next = self.next.wrapping_add(1);
        Some(id)
    }
}

pub fn horn_in_range(player: Vec3, player_speed: f32, position: Vec3, forward: Vec3) -> bool {
    if !player_speed.is_finite() || player_speed <= 10.0 || !player.is_finite() || !position.is_finite() {
        return false;
    }
    let delta = player - position;
    let distance = delta.length();
    if distance > 20.0 {
        return false;
    }
    if distance < 3.0 {
        return true;
    }
    let direction = Vec3::new(delta.x, delta.y, 0.0).normalize_or_zero();
    let forward = Vec3::new(forward.x, forward.y, 0.0).normalize_or_zero();
    let dot = direction.dot(forward);
    let cone = 40.0f32.to_radians().cos();
    dot > cone || (dot < -cone && distance < 10.0)
}

pub(super) fn horn_fade(age: f32, duration: f32) -> f32 {
    (1.0 - (age - duration).max(0.0) / 0.15).clamp(0.0, 1.0)
}

pub(super) fn random(state: &mut u32) -> f32 {
    *state = state.wrapping_mul(1664525).wrapping_add(1013904223);
    (*state >> 8) as f32 / 16777216.0
}
