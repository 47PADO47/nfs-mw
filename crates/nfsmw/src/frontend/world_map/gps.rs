//! Activating a blip's GPS route. A STUB: the route is not set yet (the pathfinder's GPS target,
//! `docs/specs/ai-pathfinder.md` section 7, is the next step). The map closes after the call either way.

use super::PlacedBlip;

/// Sets the GPS target to `blip`. Does nothing yet.
pub fn activate_gps(blip: &PlacedBlip) {
    log::info!("world map: GPS to {} at {:?} is not set yet (stub)", blip.style, blip.world);
}
