//! Fade fog lights when the bounded set changes. Surface lights remain independent.
use blackbox_gfx::LocalLight;
use glam::Vec3;

/// Fog lights at once: up to 64 current lights plus those fading out.
const CAPACITY: usize = 128;

#[derive(Clone, Copy)]
struct Entry {
    light: LocalLight,
    weight: f32,
}

#[derive(Default)]
pub struct FogLights {
    entries: Vec<Entry>,
    initialized: bool,
}

impl FogLights {
    pub fn update(&mut self, lights: &[LocalLight], limit: usize, dt: f32) -> Vec<LocalLight> {
        if limit == 0 {
            self.entries.clear();
            self.initialized = false;
            return Vec::new();
        }
        let dt = dt.clamp(0.0, 0.1);
        let mut matched = vec![false; self.entries.len()];
        // Current lights in order, each marked when it starts fading in this frame.
        let mut current = Vec::new();
        for light in lights.iter().filter(|light| light.valid()).take(limit.min(64)) {
            let position = Vec3::from_array(light.position);
            let direction = Vec3::from_array(light.direction).normalize_or_zero();
            let nearest = self
                .entries
                .iter()
                .enumerate()
                .filter(|(i, entry)| {
                    !matched[*i]
                        && position.distance_squared(Vec3::from_array(entry.light.position)) < 4.0
                        && direction.dot(Vec3::from_array(entry.light.direction).normalize_or_zero()) > 0.8
                        && light.upper_plane.is_some() == entry.light.upper_plane.is_some()
                })
                .min_by(|(_, a), (_, b)| {
                    position
                        .distance_squared(Vec3::from_array(a.light.position))
                        .total_cmp(&position.distance_squared(Vec3::from_array(b.light.position)))
                });
            let (weight, newcomer) = match nearest {
                Some((i, entry)) => {
                    matched[i] = true;
                    ((entry.weight + dt / 0.35).min(1.0), false)
                }
                None if !self.initialized => (1.0, false),
                None => ((dt / 0.35).min(1.0), true),
            };
            current.push((Entry { light: *light, weight }, newcomer));
        }
        let fading: Vec<_> = self
            .entries
            .iter()
            .enumerate()
            .filter(|(i, entry)| !matched[*i] && entry.weight > dt / 0.25)
            .map(|(_, entry)| Entry { light: entry.light, weight: entry.weight - dt / 0.25 })
            .collect();
        // Kept and fading lights always fit (they were all in the previous set). Newcomers take the room left and
        // the rest wait, so no light ever vanishes without its fade.
        let kept = current.iter().filter(|(_, newcomer)| !newcomer).count();
        let mut room = CAPACITY.saturating_sub(kept + fading.len());
        let mut next: Vec<_> = current
            .into_iter()
            .filter(|(_, newcomer)| {
                if !newcomer {
                    return true;
                }
                let admit = room > 0;
                room = room.saturating_sub(1);
                admit
            })
            .map(|(entry, _)| entry)
            .collect();
        next.extend(fading);
        self.initialized = true;
        self.entries = next;
        self.entries
            .iter()
            .map(|entry| {
                let mut light = entry.light;
                light.color = light.color.map(|component| component * entry.weight);
                light
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn light(x: f32) -> LocalLight {
        LocalLight {
            position: [x, 0.0, 3.0],
            range: 20.0,
            direction: [0.0, 0.0, -1.0],
            outer_cos: 0.2,
            inner_cos: 0.7,
            color: [1.0; 3],
            upper_plane: None,
            casts_shadow: false,
            shadow_exclude: None,
        }
    }

    #[test]
    fn changing_selection_fades_both_lights_and_disable_clears() {
        let mut selection = FogLights::default();
        let a = light(0.0);
        let b = light(10.0);
        assert_eq!(selection.update(&[a], 1, 0.016)[0].color[0], 1.0);
        let transition = selection.update(&[b], 1, 0.05);
        assert_eq!(transition.len(), 2);
        assert!(transition[0].color[0] > 0.0 && transition[0].color[0] < 0.2);
        assert!(transition[1].color[0] > 0.7 && transition[1].color[0] < 1.0);
        for _ in 0..10 {
            selection.update(&[b], 1, 0.05);
        }
        assert_eq!(selection.update(&[b], 1, 0.05)[0].color[0], 1.0);
        assert!(selection.update(&[], 0, 0.05).is_empty());
    }

    #[test]
    fn extended_budget_keeps_64_lights_and_room_for_crossfades() {
        let mut selection = FogLights::default();
        let before: Vec<_> = (0..64).map(|i| light(i as f32 * 5.0)).collect();
        let after: Vec<_> = (0..64).map(|i| light(i as f32 * 5.0 + 1000.0)).collect();
        assert_eq!(selection.update(&before, 64, 0.016).len(), 64);
        let mixed = selection.update(&after, 64, 0.05);
        assert_eq!(mixed.len(), 128, "new and fading lights must coexist");
        assert!(mixed[..64].iter().all(|light| light.color[0] < 0.2));
        assert!(mixed[64..].iter().all(|light| light.color[0] > 0.7));
        assert!(selection.update(&[], 0, 0.05).is_empty());
    }

    #[test]
    fn a_full_set_defers_newcomers_instead_of_dropping_fading_lights() {
        let mut selection = FogLights::default();
        let set = |offset: f32| -> Vec<_> { (0..64).map(|i| light(i as f32 * 5.0 + offset)).collect() };
        selection.update(&set(0.0), 64, 0.016);
        assert_eq!(selection.update(&set(1000.0), 64, 0.1).len(), 128);
        let full = selection.update(&set(2000.0), 64, 0.01);
        assert_eq!(full.len(), 128, "the fading lights keep their places");
        assert!(full.iter().all(|light| light.position[0] < 2000.0), "newcomers wait for room");
        for _ in 0..10 {
            selection.update(&set(2000.0), 64, 0.05);
        }
        let settled = selection.update(&set(2000.0), 64, 0.05);
        assert_eq!(settled.len(), 64);
        assert!(settled.iter().all(|light| light.position[0] >= 2000.0));
    }
}
