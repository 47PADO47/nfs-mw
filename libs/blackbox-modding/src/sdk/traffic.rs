//! `Traffic`: the traffic cars that mods added, listed and placed on the road (catalogue: docs/modding/api-traffic.md).

use super::ModResult;
use super::ffi::traffic::{self as host_traffic, TrafficFns};
use crate::abi::BB_TRAFFIC_FIELD;

/// One car of the traffic pool and the numbers it appears with.
#[derive(Debug, Clone, PartialEq)]
pub struct TrafficCar {
    /// The car's id (its folder), upper case.
    pub id: String,
    /// The class name: `compact`, `sedan`, `coupe`, `suv`, `pickup`, `van`, `truck`, `bus` or `taxi`.
    pub class: String,
    /// The spawn weight and share cap, percent.
    pub weight: u32,
    /// At most this many at once; 0 is no limit.
    pub max_instances: u32,
    /// The factor on the pattern's cruising speeds.
    pub speed_factor: f32,
    /// The traffic patterns the car appears in; empty is every pattern.
    pub patterns: Vec<String>,
}

impl TrafficCar {
    /// One line of the host's list. `None` for a line of fewer than five fields or a number that does not read; a field a
    /// later minor appends is ignored.
    fn parse(line: &str) -> Option<Self> {
        let mut fields = line.split(BB_TRAFFIC_FIELD);
        let id = fields.next().filter(|id| !id.is_empty())?.to_owned();
        let class = fields.next()?.to_owned();
        let weight = fields.next()?.parse().ok()?;
        let max_instances = fields.next()?.parse().ok()?;
        let speed_factor = fields.next()?.parse().ok()?;
        let patterns = fields.next().unwrap_or_default().split(',').filter(|name| !name.is_empty());
        Some(Self { id, class, weight, max_instances, speed_factor, patterns: patterns.map(str::to_owned).collect() })
    }
}

/// The `bb.traffic` interface of the host, as a mod uses it. Obtained from `Ctx::traffic`.
#[derive(Clone, Copy)]
pub struct Traffic {
    fns: TrafficFns,
}

impl Traffic {
    pub(crate) fn new(fns: TrafficFns) -> Self {
        Traffic { fns }
    }

    /// The traffic car pool: the cars mods added with `role = "traffic"`, in the order they were registered. A pool read
    /// before a mod's cars are in (the 2012 mod converts on a worker thread) is shorter; read it again later.
    pub fn cars(&self) -> ModResult<Vec<TrafficCar>> {
        let text = host_traffic::list(&self.fns)?;
        Ok(text.lines().filter_map(TrafficCar::parse).collect())
    }

    /// Ask for the traffic car `car` (an id of the pool, any case) at `position` (x forward, y left; the height is not
    /// used), heading `heading` radians from +x. The car is placed on the nearest lane that can take it, after the mods run,
    /// as soon as its model is loaded (a few seconds the first time); a request that cannot be placed in 30 seconds is
    /// dropped. `NotFound` for a car that is not in the pool, `NotReady` while no world is loaded.
    pub fn spawn(&self, car: &str, position: [f32; 3], heading: f32) -> ModResult<()> {
        host_traffic::spawn(&self.fns, car, position, heading)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_line_of_the_list_is_a_car() {
        let car = TrafficCar::parse("AUDA3_TR\tcompact\t10\t0\t1\tdowntown,collegesouth").unwrap();
        assert_eq!(
            (car.id.as_str(), car.class.as_str(), car.weight, car.max_instances),
            ("AUDA3_TR", "compact", 10, 0)
        );
        assert_eq!(car.speed_factor, 1.0);
        assert_eq!(car.patterns, ["downtown", "collegesouth"]);
    }

    #[test]
    fn a_car_with_no_pattern_has_an_empty_last_field() {
        let car = TrafficCar::parse("GMCTOPKIC_TR\ttruck\t4\t1\t0.85\t").unwrap();
        assert!(car.patterns.is_empty());
        assert_eq!((car.max_instances, car.speed_factor), (1, 0.85));
    }

    #[test]
    fn fields_a_later_minor_appends_are_ignored() {
        let car = TrafficCar::parse("A_TR\tsedan\t20\t0\t1\t\tnext\tfields").unwrap();
        assert_eq!(car.id, "A_TR");
    }

    #[test]
    fn a_line_that_does_not_read_is_no_car() {
        assert!(TrafficCar::parse("").is_none());
        assert!(TrafficCar::parse("A_TR\tsedan").is_none());
        assert!(TrafficCar::parse("A_TR\tsedan\tmany\t0\t1\t").is_none());
        assert!(TrafficCar::parse("\tsedan\t20\t0\t1\t").is_none(), "no id");
    }
}
