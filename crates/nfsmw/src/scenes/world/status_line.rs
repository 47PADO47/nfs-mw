//! The one-line status of the world scene (`--screenshot` prints it).
use super::*;

impl WorldScene {
    pub(super) fn status_line(&self) -> Option<String> {
        let (resident, loading, shared) = self.residency.counts();
        let (eligible, selected) = self.residency.shadow_support_counts();
        let p = match (self.players.lead(), self.view) {
            (Some(drive), View::Chase) => drive.position(),
            _ => self.camera.position,
        };
        let zone = self.residency.zone().unwrap_or_else(|| "-".into());
        let markers = match (self.players.lead(), self.markers_on) {
            (Some(drive), true) => {
                let (walls, tyres) = drive.marker_counts();
                format!(", {walls} contacts, {tyres} tyre rays")
            }
            _ => String::new(),
        };
        Some(format!(
            "{}zone {zone}, tiles {resident} (+{loading} loading), shadows {selected}/{eligible} extra, {} drawn, {} props ({} knocked over), at ({:.0}, {:.0}, {:.0}){markers}",
            if shared { "" } else { "loading shared sets… " },
            self.visible.len(),
            self.residency.props().len(),
            self.residency.props().knocked_count(),
            p.x,
            p.y,
            p.z
        ))
    }
}
