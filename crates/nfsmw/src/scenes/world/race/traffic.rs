//! Ambient traffic follows event percentages and patterns without rescaling the actor budget.

use nfsmw_data::gameplay::{RaceParams, RaceType};

use super::RaceSession;

impl RaceSession {
    pub(in crate::scenes::world) fn traffic_context(&self) -> (u32, Option<&str>) {
        let params = &self.origin.race.params;
        let density = match self.kind {
            // Scripted drag traffic has its own route and spawning adapter.
            RaceType::Drag => 0,
            _ => density(params, self.options.traffic),
        };
        (density, params.traffic_pattern.as_deref())
    }
}

fn density(params: &RaceParams, selected: Option<i32>) -> u32 {
    let stored = match params.force_traffic_density > 0 {
        true => params.force_traffic_density,
        false => params.traffic_level,
    };
    selected.unwrap_or(stored).clamp(0, 100) as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn traffic_parity_event_density_uses_the_stored_level_and_explicit_options() {
        let params = RaceParams { traffic_level: 30, ..Default::default() };
        assert_eq!(density(&params, None), 30);
        let forced = RaceParams { force_traffic_density: 45, ..params };
        assert_eq!(density(&forced, None), 45);
        assert_eq!(density(&forced, Some(10)), 10);
        assert_eq!(density(&forced, Some(-10)), 0);
        assert_eq!(density(&forced, Some(150)), 100);
    }
}
