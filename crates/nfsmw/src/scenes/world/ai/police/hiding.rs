//! The player's hiding spots in the world: which HIDDEN track zones hold the player's feet, the latch that turns that
//! into "hidden" (`blackbox_pursuit::hiding`), and the console report. Child of `police`. Spec:
//! `docs/specs/pursuit-hiding.md`.

use blackbox_pursuit::hiding::{self, Hiding, Scope};
use blackbox_roads::from_zone_space;

use super::*;

/// The player's hiding state and the height of its body, which locates the feet.
pub(super) struct HidingState {
    pub(super) latch: Hiding,
    /// Half the height of the player's car, metres: the body origin is its middle, the feet are this far below.
    pub(super) half_height: f32,
}

impl Default for HidingState {
    fn default() -> Self {
        Self { latch: Hiding::default(), half_height: 0.5 }
    }
}

/// Which pursuers a zone hides the player from, by its first data word (`"Car"`, `"Heli"` or anything else).
fn scope_of(zone: &TrackZone) -> Scope {
    match zone.data[0] as u32 {
        CAR_HASH => Scope::Cars,
        HELI_HASH => Scope::Helicopters,
        _ => Scope::Both,
    }
}

/// The map position (x east, y north) and the physics height of a zone's middle.
fn middle(zone: &TrackZone) -> Vec3 {
    let sum: Vec2 = zone.polygon.iter().copied().sum();
    from_zone_space(sum / zone.polygon.len().max(1) as f32, zone.elevation)
}

fn icon_position(zone: &TrackZone) -> Vec3 {
    super::super::super::space::to_render(from_zone_space(zone.position, zone.elevation).to_array())
}

impl super::super::TrafficWorld {
    /// Authored hiding-zone centres in render space, for world icon placement.
    pub(in crate::scenes::world) fn hiding_icon_positions(&self) -> impl Iterator<Item = Vec3> + '_ {
        self.police.hidden_zones.iter().map(icon_position)
    }
    /// Tells the police how tall the player's car is (half its height, metres), so its feet can be found.
    pub fn note_player_height(&mut self, half_height: f32) {
        self.police.hiding.half_height = half_height;
    }

    /// The scopes of the HIDDEN zones that hold a player whose body is at `at` (physics space), in file order.
    fn hiding_spots(&self, at: Vec3) -> Vec<Scope> {
        let feet = at.y - self.police.hiding.half_height;
        let point = to_zone_space(at);
        self.police
            .hidden_zones
            .iter()
            .filter(|z| z.contains(point) && hiding::at_height(z.elevation, feet))
            .map(scope_of)
            .collect()
    }

    /// Advances the latch by `dt` seconds with the player's body at `at` (physics space).
    pub(super) fn update_hiding(&mut self, dt: f32, at: Vec3) {
        let spots = self.hiding_spots(at);
        let evade = self.is_pursuing().then(|| self.police.evade_level());
        let unseen = hiding::unseen_on_entry(evade);
        self.police.hiding.latch.update(dt, &spots, unseen);
    }

    /// Whether the spots hide the player from the ground cars now (the latched flag, not the bare zone test).
    pub(in crate::scenes::world::ai) fn hidden_from_cars(&self) -> bool {
        self.police.hiding.latch.from_cars()
    }

    /// How far the latch of the spot the player is in has filled, 0 to 1, for the HUD's hiding mark.
    pub fn hiding_fill(&self) -> Option<f32> {
        self.police.hiding.latch.how_hidden()
    }

    /// The `pursuit hiding` report: the spots of the track nearest to `at` (physics space) and the player's latch.
    pub fn hiding_report(&self, at: Vec3, listed: usize) -> String {
        let zones = &self.police.hidden_zones;
        let mut lines = vec![format!("{} hiding spots on this track", zones.len())];
        let mut near: Vec<(f32, &TrackZone)> =
            zones.iter().map(|z| (Vec2::new(middle(z).x - at.x, middle(z).z - at.z).length(), z)).collect();
        near.sort_by(|a, b| a.0.total_cmp(&b.0));
        let feet = at.y - self.police.hiding.half_height;
        for (metres, zone) in near.iter().take(listed) {
            let mid = middle(zone);
            let extent = zone.bbox_max - zone.bbox_min;
            let holds = zone.contains(to_zone_space(at));
            let state = match (holds, hiding::at_height(zone.elevation, feet)) {
                (true, true) => "  PLAYER INSIDE".to_owned(),
                (true, false) => format!("  under or over it ({:+.1} m)", feet - zone.elevation),
                _ => String::new(),
            };
            lines.push(format!(
                "  map ({:.0}, {:.0}) height {:.1} m, {:.0} x {:.0} m, hides from {:?}, {metres:.0} m away{state}",
                mid.z,
                -mid.x,
                zone.elevation,
                extent.x,
                extent.y,
                scope_of(zone),
            ));
        }
        let latch = &self.police.hiding.latch;
        lines.push(format!(
            "player: {}, hidden from cars {}, from helicopters {}, fill {}",
            match latch.in_spot() {
                true => "in a spot",
                false => "not in a spot",
            },
            latch.from_cars(),
            latch.from_helicopters(),
            latch.how_hidden().map_or("-".to_owned(), |f| format!("{f:.2}")),
        ));
        lines.join("\n")
    }
}

#[cfg(test)]
mod icon_tests {
    use super::*;

    #[test]
    fn hiding_icon_uses_authored_anchor_not_polygon_average() {
        let zone = TrackZone {
            kind: 8,
            position: Vec2::new(12.0, 23.0),
            direction: Vec2::X,
            elevation: 147.8,
            visit_info: 0,
            data: [0; 4],
            bbox_min: Vec2::ZERO,
            bbox_max: Vec2::splat(100.0),
            polygon: vec![Vec2::ZERO, Vec2::new(100.0, 0.0), Vec2::splat(100.0)],
        };
        assert_eq!(icon_position(&zone), Vec3::new(12.0, 23.0, 147.8));
        assert_ne!(icon_position(&zone), super::super::super::super::space::to_render(middle(&zone).to_array()));
    }
}
