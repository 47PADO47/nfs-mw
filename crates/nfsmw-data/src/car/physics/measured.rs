//! A car's measured performance, kept for the session.
//!
//! `blackbox_vehicle::performance::measure_with_integration` drives the car flat out (up to 5,400 steps), so a call
//! costs tens of milliseconds. The car lot, Quick Play's pool, the racers of a race and the cop cars all measure the same
//! cars again and again; this keeps each result for the rest of the process.
//!
//! - [`performance`]: by car type name. The entries belong to the physics files (`ATTRIBUTES.BIN`, `GLOBALB.LZC`), as
//!   the session memo (`files.rs`) keys them, so a changed or replaced file starts a new set. For callers that would
//!   otherwise build the car's physics first.
//! - [`performance_of_spec`]: by the spec itself, for callers that already have it and no install at hand. The key is the
//!   spec's full description, so a changed spec never hits an old entry.

use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard};

use blackbox_vehicle::performance::{Performance, estimate_ai, measure_with_integration};
use blackbox_vehicle::tires::TireIntegration;
use blackbox_vehicle::vehicle::VehicleSpec;
use game_install::GameDir;

use crate::{memo, shared};

/// The files a car's performance comes from: the `pvehicle` records and the collision bounds.
const FILES: [&str; 2] = [shared::ATTRIBUTES_FILE, shared::GLOBALB_FILE];

/// The measured cars of one set of physics files, by lower-case type name. `None` is a car whose physics cannot be read.
#[derive(Default)]
struct Measured {
    by_car: Mutex<HashMap<String, Option<Performance>>>,
}

impl Measured {
    fn cars(&self) -> MutexGuard<'_, HashMap<String, Option<Performance>>> {
        self.by_car.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// The stored result for `type_name`, else `measure`'s, which is stored. `measure` runs without the lock held, so
    /// several threads can measure different cars at once; if two race on one car, the first result stored wins.
    fn get_or_measure(&self, type_name: &str, measure: impl FnOnce() -> Option<Performance>) -> Option<Performance> {
        let key = type_name.to_ascii_lowercase();
        let hit = self.cars().get(&key).copied();
        if let Some(hit) = hit {
            return hit;
        }
        let measured = measure();
        *self.cars().entry(key).or_insert(measured)
    }
}

/// The performance of car type `type_name`, measured by `measure` the first time the type is asked for with the current
/// physics files (`None` from `measure` means the car's physics cannot be read, and is kept too).
pub fn performance(
    dir: &GameDir,
    type_name: &str,
    measure: impl FnOnce() -> Option<Performance>,
) -> Option<Performance> {
    let Ok(set) = memo(dir, &FILES, || Ok(Measured::default())) else { return measure() };
    set.get_or_measure(type_name, measure)
}

/// Action-planning estimates have their own cache identity, independent of physical launch measurements.
pub fn ai_performance(
    dir: &GameDir,
    type_name: &str,
    estimate: impl FnOnce() -> Option<Performance>,
) -> Option<Performance> {
    let Ok(set) = memo(dir, &FILES, || Ok(Measured::default())) else { return estimate() };
    set.get_or_measure(&format!("ai-estimate-v1/{type_name}"), estimate)
}

/// The results of [`performance_of_spec`], by the description of the spec and the integration policy.
static BY_SPEC: Mutex<Option<HashMap<String, Performance>>> = Mutex::new(None);

/// The performance of `spec`, measured once per distinct spec in the process.
pub fn performance_of_spec(spec: &VehicleSpec, integration: TireIntegration) -> Performance {
    let key = format!("{integration:?}|{spec:?}");
    let hit = lock_specs().as_ref().and_then(|map| map.get(&key).copied());
    if let Some(hit) = hit {
        return hit;
    }
    let measured = measure_with_integration(spec, integration);
    *lock_specs().get_or_insert_with(HashMap::new).entry(key).or_insert(measured)
}

/// The analytical AI curve; unsupported parameter sets retain the measured fallback.
/// This changes action planning only, never the physical vehicle's engine or tires.
pub fn ai_performance_of_spec(spec: &VehicleSpec, integration: TireIntegration) -> Performance {
    let key = format!("ai-estimate-v1|{integration:?}|{spec:?}");
    let hit = lock_specs().as_ref().and_then(|map| map.get(&key).copied());
    if let Some(hit) = hit {
        return hit;
    }
    let estimated = estimate_ai(spec).unwrap_or_else(|| performance_of_spec(spec, integration));
    *lock_specs().get_or_insert_with(HashMap::new).entry(key).or_insert(estimated)
}

fn lock_specs() -> MutexGuard<'static, Option<HashMap<String, Performance>>> {
    BY_SPEC.lock().unwrap_or_else(|e| e.into_inner())
}

#[cfg(test)]
#[path = "measured_tests.rs"]
mod tests;
