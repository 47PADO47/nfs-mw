//! The race on the minimap: the next checkpoint and the other racers.

use blackbox_race::Phase;
use glam::Vec3;

use super::flow::next_gate;
use crate::hud::{CHECKPOINT, MapBlip, RACER};
use crate::scenes::world::{WorldScene, space};

impl WorldScene {
    /// Appends the race's blips: the player's next gate (`MMICON_CHECKPOINT`) and the racers still running
    /// (`MMICON_AIRACER_n`), in the minimap's render space.
    pub(in crate::scenes::world) fn race_blips(&self, out: &mut Vec<MapBlip>) {
        let Some(engine) = self.race.as_ref().and_then(|s| s.engine.as_ref()) else { return };
        let racers = engine.racers();
        let Some(me) = racers.first() else { return };
        if engine.phase() == Phase::Over || me.done() {
            return;
        }
        if let Some(gate) = next_gate(engine.course(), me.progress.reached) {
            out.push(gate_blip(gate));
        }
        let Some(traffic) = self.traffic.as_ref() else { return };
        for (id, racer) in racers.iter().enumerate().skip(1).filter(|(_, r)| !r.done()) {
            let at = space::to_render(racer.position.to_array());
            let heading = traffic.racer_heading(id);
            out.push(MapBlip { style: RACER, position: [at.x, at.y], active: true, heading });
        }
    }
}

/// A checkpoint blip at a gate's centre (physics space).
fn gate_blip(centre: Vec3) -> MapBlip {
    let at = space::to_render(centre.to_array());
    MapBlip { style: CHECKPOINT, position: [at.x, at.y], active: true, heading: None }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_checkpoint_blip_is_active_and_in_render_space() {
        // Physics (-4404.865, 209.6, 1511.8) is render (1511.8, 4404.9, 209.6).
        let blip = gate_blip(Vec3::new(-4404.865, 209.6, 1511.8));
        assert_eq!(blip.style, CHECKPOINT);
        assert!(blip.active && blip.heading.is_none());
        assert_eq!(blip.position, [1511.8, 4404.865]);
    }
}
