//! Bounded scenery requests for shadows that reach beyond the colour PVS.
use super::{MAX_IN_FLIGHT, Residency, TileState};
use blackbox_gfx::RenderBackend;

pub(super) const MAX_SHADOW_IN_FLIGHT: usize = 2;
pub(super) const MAX_EXTRA_TILES: usize = 32;
const UPSTREAM_SUPPORT: f32 = 180.0;
const RETENTION_PREFERENCE: f32 = 32.0;

impl Residency {
    /// The full receiver disk is swept upstream; aiming the camera does not change support.
    pub(in crate::scenes::world) fn set_shadow_sun(&mut self, to_sun: glam::Vec3) {
        self.shadow_direction = to_sun.is_finite().then(|| {
            let horizontal = to_sun.truncate().normalize_or_zero();
            horizontal.to_array()
        });
    }

    /// Perspective far corners and the camera-to-streaming-focus offset enlarge the receiver disk.
    pub(in crate::scenes::world) fn set_shadow_receiver_extent(&mut self, scale: f32, offset: f32) {
        self.shadow_extent = [
            match scale.is_finite() {
                true => scale.clamp(1.0, 16.0),
                false => 1.0,
            },
            match offset.is_finite() {
                true => offset.clamp(0.0, 960.0),
                false => 0.0,
            },
        ];
    }

    /// Eligible extra sections and selected extra sections; core colour tiles are excluded.
    pub(in crate::scenes::world) fn shadow_support_counts(&self) -> (usize, usize) {
        (self.shadow_eligible, self.shadow_wanted.len())
    }

    pub(super) fn update_shadow_support(&mut self, renderer: &dyn RenderBackend, focus: [f32; 2]) {
        let old_counts = self.shadow_support_counts();
        let settings = renderer.render_experiments().sanitized();
        let distance = settings.caster_distance();
        self.shadow_range = match settings.sun_shadows && distance.is_finite() && distance > 0.0 {
            true => distance.min(960.0),
            false => 0.0,
        };
        let mut previous = std::mem::take(&mut self.shadow_wanted);
        let mut candidates = std::mem::take(&mut self.shadow_candidates);
        candidates.clear();
        self.shadow_eligible = 0;
        if self.shadow_range <= 0.0 {
            previous.clear();
            self.shadow_wanted = previous;
            self.shadow_candidates = candidates;
            return;
        }
        for (i, section) in self.sections.iter().enumerate() {
            if !section.is_spatial() || self.wanted.contains(&i) {
                continue;
            }
            let retained = previous.contains(&i) || matches!(self.tiles.get(&i), Some(TileState::Resident { .. }));
            let center = glam::Vec2::from_array(section.center) - glam::Vec2::from_array(focus);
            let support = |margin: f32| {
                let receiver_range = self.shadow_range * self.shadow_extent[0] + self.shadow_extent[1];
                let (ray, radius) = match self.shadow_direction {
                    Some(direction) => {
                        (glam::Vec2::from_array(direction) * UPSTREAM_SUPPORT * margin, receiver_range * margin)
                    }
                    None => (glam::Vec2::ZERO, (receiver_range + UPSTREAM_SUPPORT) * margin),
                };
                let t = match ray.length_squared() > 0.0 {
                    true => (center.dot(ray) / ray.length_squared()).clamp(0.0, 1.0),
                    false => 0.0,
                };
                center.distance(ray * t) - section.radius <= radius
            };
            if !support(1.0) && !(retained && support(1.25)) {
                continue;
            }
            let preference = match retained {
                true => RETENTION_PREFERENCE,
                false => 0.0,
            };
            candidates.push((i, section.distance_to(focus[0], focus[1]) - preference));
        }
        candidates.sort_unstable_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
        self.shadow_eligible = candidates.len();
        previous.clear();
        previous.extend(candidates.iter().take(MAX_EXTRA_TILES).map(|&(i, _)| i));
        self.shadow_wanted = previous;
        self.shadow_candidates = candidates;
        let counts = self.shadow_support_counts();
        if counts != old_counts && counts.0 > counts.1 {
            log::debug!("shadow streaming support limited to {} of {} eligible extra sections", counts.1, counts.0);
        }
    }

    pub(super) fn request_shadow_support(&mut self) {
        let in_flight = self.tiles.values().filter(|tile| matches!(tile, TileState::Requested)).count();
        let shadow_in_flight = self
            .tiles
            .iter()
            .filter(|(i, tile)| !self.wanted.contains(i) && matches!(tile, TileState::Requested))
            .count();
        let free = MAX_IN_FLIGHT.saturating_sub(in_flight).min(MAX_SHADOW_IN_FLIGHT.saturating_sub(shadow_in_flight));
        let missing: Vec<_> =
            self.shadow_wanted.iter().copied().filter(|i| !self.tiles.contains_key(i)).take(free).collect();
        for i in missing {
            self.tiles.insert(i, TileState::Requested);
            self.streamer.request(i);
        }
    }
}
