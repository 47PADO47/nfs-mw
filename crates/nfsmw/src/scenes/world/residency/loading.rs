//! Dispatch section worker results without conflating collision and colour readiness.
use super::*;

impl Residency {
    pub(super) fn handle(&mut self, event: StreamerEvent) {
        match event {
            StreamerEvent::Collision { index, result } => self.receive_traffic_collision(index, result),
            StreamerEvent::Loaded(data) if !self.sections[data.index].is_spatial() => self.shared_pending.push(*data),
            StreamerEvent::Loaded(data) => self.arrived.push_back(*data),
            StreamerEvent::Failed { index, error } => {
                log::error!("section {}: {error}", self.sections[index].name);
                if self.sections[index].is_spatial() {
                    self.tiles.insert(
                        index,
                        TileState::Resident { resources: Default::default(), placed: vec![], packs: vec![] },
                    );
                    return;
                }
                self.shared_total -= 1;
            }
        }
    }
}
