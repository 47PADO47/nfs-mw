//! The traffic cars that mods add (`role = "traffic"`, docs/modding/api-traffic.md): their rules in the neighbourhood patterns,
//! the speed they cruise at, and the cars a mod or the console asks for by name.
//!
//! A mod traffic car is one more record in the car list of each pattern it appears in, after the install's own records, with
//! the `rate`, `max_instances` and `percent` of its weight (`nfsmw_data::car::mods::traffic`). Everything else is the
//! manager's: the density tables, the population budget, the cop share and the shares of the other types act on it as on any
//! traffic car.

use blackbox_collision::CollisionWorld;
use nfsmw_data::car::mods::{TrafficRule, traffic_rules, traffic_unit};
use nfsmw_data::car::physics::SurfaceTable;

use super::manager::{MAX_MOD_RECORDS, Rule};
use super::traffic::Cruise;
use super::{Focus, TrafficWorld};

/// The rules the mod traffic cars add to the pattern `pattern`: at most [`MAX_MOD_RECORDS`] of them.
pub(super) fn rules_for(pattern: &str) -> Vec<Rule> {
    rules_of(traffic_rules(pattern))
}

fn rules_of(rules: Vec<TrafficRule>) -> Vec<Rule> {
    rules
        .into_iter()
        .take(MAX_MOD_RECORDS)
        .map(|rule| Rule { name: rule.name, rate: rule.rate, max_instances: rule.max_instances, percent: rule.percent })
        .collect()
}

/// The cruising speeds of a car of the model `model`: the pattern's, times the car's speed factor when it is a mod traffic car.
pub(super) fn scaled(cruise: Cruise, model: &str) -> Cruise {
    let Some(unit) = traffic_unit(model) else { return cruise };
    Cruise { street: cruise.street * unit.speed_factor, highway: cruise.highway * unit.speed_factor, ..cruise }
}

impl TrafficWorld {
    /// Serves the cars asked for by name (`crate::traffic_requests`): a request whose model is loaded is placed at its point
    /// (the nearest lane that can take a car), one whose model is not loaded asks for it and waits, and one that cannot be
    /// placed waits and tries again until its time is up. A model that failed to load drops its requests.
    pub(super) fn serve_asked_cars(
        &mut self,
        tick: f32,
        focus: Focus,
        collision: &CollisionWorld,
        surfaces: &SurfaceTable,
    ) {
        let asked = crate::traffic_requests::take();
        if asked.is_empty() {
            return;
        }
        let mut waiting = Vec::new();
        for mut ask in asked {
            let name = ask.car.to_ascii_lowercase();
            if self.failed.contains(&name) {
                log::warn!("traffic: {} cannot be placed: its model did not load", ask.car);
                continue;
            }
            let placed = match self.models.iter().position(|m| m.name.eq_ignore_ascii_case(&ask.car)) {
                Some(index) => self.spawn_model_at(index, ask.at, ask.heading, focus, collision, surfaces),
                None => {
                    self.request_traffic_model(&ask.car);
                    false
                }
            };
            if placed {
                continue;
            }
            ask.seconds_left -= tick;
            if ask.seconds_left <= 0.0 {
                log::warn!("traffic: {} was not placed at ({:.0}, {:.0}) in time", ask.car, ask.at[0], ask.at[1]);
                continue;
            }
            waiting.push(ask);
        }
        crate::traffic_requests::put_back(waiting);
    }

    /// One line per mod traffic car for the `traffic cars` console command: its unit, and whether its model is loaded.
    pub fn mod_traffic_status(&self) -> String {
        let cars = nfsmw_data::car::mods::traffic_cars();
        if cars.is_empty() {
            return "no mod adds a traffic car".to_owned();
        }
        let lines: Vec<String> = cars
            .iter()
            .filter_map(|car| {
                let unit = car.traffic.as_ref()?;
                let loaded = self.models.iter().any(|m| m.name.eq_ignore_ascii_case(&car.folder));
                let state = match (loaded, self.failed.contains(&car.folder.to_ascii_lowercase())) {
                    (_, true) => "failed to load",
                    (true, false) => "loaded",
                    (false, false) => "not loaded yet",
                };
                Some(format!("{}: {} ({state})", car.folder, unit.describe()))
            })
            .collect();
        lines.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule(name: &str) -> TrafficRule {
        TrafficRule { name: name.to_owned(), rate: 6.0, max_instances: 1, percent: 20 }
    }

    #[test]
    fn a_rule_of_the_data_is_a_rule_of_the_manager() {
        let rules = rules_of(vec![rule("hatch")]);
        assert_eq!(
            rules,
            [Rule { name: "hatch".to_owned(), rate: 6.0, max_instances: 1, percent: 20 }],
            "the numbers pass unchanged"
        );
    }

    #[test]
    fn no_more_than_the_cap_of_mod_rules_are_taken() {
        let many: Vec<TrafficRule> = (0..MAX_MOD_RECORDS + 10).map(|n| rule(&format!("car{n}"))).collect();
        assert_eq!(rules_of(many).len(), MAX_MOD_RECORDS);
    }

    #[test]
    fn a_car_that_is_not_a_mod_traffic_car_cruises_at_the_patterns_speeds() {
        let cruise = Cruise::default();
        assert_eq!(scaled(cruise, "trafha"), cruise);
    }
}
