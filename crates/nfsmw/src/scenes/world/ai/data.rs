//! Setting the traffic up from the install's data: the patterns of the neighbourhoods and the patrol cops.

use blackbox_attrib::Database;
use blackbox_roads::{TrackZones, to_zone_space};
use nfsmw_data::pursuit::{ground_cops, heat_row, patrol_speeds_mph};
use nfsmw_data::traffic::{TrafficPattern, pattern_hash, patterns};

use super::manager::Rule;
use super::traffic::{Cruise, MPH};
use super::{PatrolPlan, Pattern, Patterns, TrafficWorld};

/// The type of the track path zones that name a traffic pattern.
const PATTERN_ZONE: u32 = 9;
/// The heat level whose wave patrols the streets: the first one.
const PATROL_HEAT: u32 = 1;
fn pattern_of(p: &TrafficPattern) -> Pattern {
    Pattern {
        name: p.name.clone(),
        cruise: Cruise::traffic(p.speed_street_mph * MPH, p.speed_highway_mph * MPH),
        rules: p
            .vehicles
            .iter()
            .map(|v| Rule { name: v.name.clone(), rate: v.rate, max_instances: v.max_instances, percent: v.percent })
            .collect(),
    }
}

/// The patterns of the install, selected by the pattern zone under the player.
pub fn build_patterns(db: &Database, zones: Option<TrackZones>) -> Option<Patterns> {
    let list: Vec<Pattern> = patterns(db).iter().map(pattern_of).collect();
    if list.is_empty() {
        return None;
    }
    let by_hash: Vec<(u32, String)> = list.iter().map(|p| (pattern_hash(&p.name), p.name.clone())).collect();
    let locate = move |at| {
        let zone = zones.as_ref()?.first_of_kind_at(PATTERN_ZONE, to_zone_space(at))?;
        let hash = zone.data[0] as u32;
        by_hash.iter().find(|(h, _)| *h == hash).map(|(_, name)| name.clone())
    };
    Some(Patterns::new(list, Box::new(locate)))
}

/// The patrol cops: the cars of the first heat level's wave, at the search-mode speeds.
pub fn patrol_plan(db: &Database) -> Option<PatrolPlan> {
    let row = heat_row(db, PATROL_HEAT)?;
    let cars: Vec<String> = ground_cops(&row).map(|c| c.name.clone()).collect();
    let (city, highway) = patrol_speeds_mph(db);
    Some(PatrolPlan::new(cars, city, highway))
}

impl TrafficWorld {
    /// Gives the traffic its patterns and patrol cops from the install.
    pub fn load_data(&mut self, db: &Database, zones: Option<TrackZones>) {
        let of_kind = |kind: u32| -> Vec<blackbox_roads::TrackZone> {
            zones.iter().flat_map(|z| z.zones.iter().filter(|z| z.kind == kind).cloned()).collect()
        };
        self.police.set_hidden_zones(of_kind(super::police::HIDDEN_ZONE));
        self.police.set_tunnel_zones(of_kind(super::police::TUNNEL_ZONE));
        self.police.set_heli_settings(crate::settings::helicopter::load());
        self.police.set_settings(crate::settings::bartender::load(), crate::settings::fairbusts::load());
        match build_patterns(db, zones) {
            Some(patterns) => self.set_patterns(patterns),
            None => log::warn!("traffic: the install has no traffic patterns, so no traffic will spawn"),
        }
        if let Some(plan) = patrol_plan(db) {
            self.set_patrol(plan);
        }
    }
}
