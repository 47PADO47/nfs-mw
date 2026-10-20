//! Where the car returns to after a bust in a career (`docs/specs/busted-aftermath.md` section 7): the respawn marker of
//! the last safe house the player visited, else of the last zone of any kind. Free roam resumes there once the player
//! leaves the safe house menu.
//!
//! The original loads a new world at the marker (`career-safehouse-2.md` section 9). The rewrite keeps the world and
//! places the car on the road nearest the marker; the heading is the road's. Nothing is remembered across sessions (the
//! original saves both keys in the gameplay blob), and with no zone visited the car stays where it is (the original
//! starts at `career_start_<region>`).

use glam::Vec3;

use super::WorldScene;
use crate::scenes::world::entry_zones::ZoneKind;

/// The respawn markers the player has visited, in render space.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub(in crate::scenes::world) struct Respawns {
    /// The marker of the last visited zone of any kind (`+0x108`).
    pub home: Option<Vec3>,
    /// The marker of the last visited safe house (`+0x10C`).
    pub safe_house: Option<Vec3>,
    /// A bust was paid: the car goes to the marker when the game runs again.
    pub pending: bool,
}

impl Respawns {
    /// The player engaged a zone whose respawn marker is `marker`.
    pub fn visit(&mut self, kind: ZoneKind, marker: Option<Vec3>) {
        let Some(marker) = marker else { return };
        self.home = Some(marker);
        if kind == ZoneKind::SafeHouse {
            self.safe_house = Some(marker);
        }
    }

    /// Where a car released from the safe house menu starts: the safe house marker, else the last zone's.
    pub fn target(&self) -> Option<Vec3> {
        self.safe_house.or(self.home)
    }
}

impl WorldScene {
    /// Remembers the respawn marker of the zone called `name`, which the player engaged.
    pub(super) fn career_visit_zone(&mut self, kind: ZoneKind, name: &str) {
        let marker = self.entry.zones.zones.iter().find(|zone| zone.name == name).and_then(|zone| zone.respawn);
        self.career.respawn.visit(kind, marker);
    }

    /// The game runs again after a paid bust: the car goes to the respawn marker.
    pub(super) fn career_respawn(&mut self) {
        if !std::mem::take(&mut self.career.respawn.pending) {
            return;
        }
        let Some(at) = self.career.respawn.target() else {
            log::info!("career: busted with no zone visited, the car stays where it is");
            return;
        };
        let Some(drive) = self.drive.as_mut() else { return };
        drive.respawn_near([at.x, at.y], None);
        log::info!("career: busted, the car goes back to the respawn marker at ({:.0}, {:.0})", at.x, at.y);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_safe_house_marker_wins_over_the_last_zone() {
        let mut respawns = Respawns::default();
        respawns.visit(ZoneKind::SafeHouse, Some(Vec3::new(1.0, 2.0, 0.0)));
        respawns.visit(ZoneKind::CarLot, Some(Vec3::new(9.0, 9.0, 0.0)));
        assert_eq!(respawns.home, Some(Vec3::new(9.0, 9.0, 0.0)));
        assert_eq!(respawns.target(), Some(Vec3::new(1.0, 2.0, 0.0)));
    }

    #[test]
    fn without_a_safe_house_the_last_zone_is_the_target() {
        let mut respawns = Respawns::default();
        assert_eq!(respawns.target(), None);
        respawns.visit(ZoneKind::AutoShop, Some(Vec3::new(5.0, 6.0, 0.0)));
        assert_eq!(respawns.target(), Some(Vec3::new(5.0, 6.0, 0.0)));
    }

    #[test]
    fn a_zone_without_a_marker_remembers_nothing() {
        let mut respawns = Respawns::default();
        respawns.visit(ZoneKind::SafeHouse, None);
        assert_eq!(respawns, Respawns::default());
    }
}
