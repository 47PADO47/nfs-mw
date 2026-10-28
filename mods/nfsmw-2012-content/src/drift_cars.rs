//! Which cars are 2012 vehicles for the drift rules (`drift_nitrous.rs`): the cars of this mod's own `cars/` manifests.
//!
//! The mod reads the id of the car the player drives from `bb.engine` (`Engine::car`, the car's folder id in the game's case)
//! and compares it with the manifests it wrote, one `cars/<id>.toml` per converted car. A stock car or a car of another mod
//! is not in the list, so the drift assist and the drift nitrous leave it alone.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// How long a list that does not hold the car is trusted before the folder is read again: the conversion may be moving the
/// cars into place while the game runs.
const RECHECK: Duration = Duration::from_secs(5);

/// Whether the car `car_id` (any case) is one of `known_ids` (any case). An empty id is no car.
pub fn applies_to<S: AsRef<str>>(car_id: &str, known_ids: &[S]) -> bool {
    if car_id.is_empty() {
        return false;
    }
    known_ids.iter().any(|known| known.as_ref().eq_ignore_ascii_case(car_id))
}

/// The ids of the cars with a manifest in `cars_dir` (`<id>.toml`, lower case). Empty when the folder does not read.
pub fn ids_in(cars_dir: &Path) -> Vec<String> {
    let entries = std::fs::read_dir(cars_dir).into_iter().flatten().flatten();
    let mut ids: Vec<String> = entries
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|extension| extension.eq_ignore_ascii_case("toml")))
        .filter_map(|path| path.file_stem().map(|stem| stem.to_string_lossy().to_ascii_lowercase()))
        .collect();
    ids.sort();
    ids
}

/// The mod's cars, read from its folder when asked.
pub struct KnownCars {
    cars_dir: PathBuf,
    ids: Vec<String>,
    read_at: Option<Instant>,
}

impl KnownCars {
    pub fn new(mod_dir: &Path) -> Self {
        Self { cars_dir: mod_dir.join("cars"), ids: Vec::new(), read_at: None }
    }

    /// Whether the player's car `car_id` is one of the mod's cars. A car that is not in the list read so far makes the folder
    /// be read again, at most once in [`RECHECK`].
    pub fn contains(&mut self, car_id: &str) -> bool {
        if car_id.is_empty() {
            return false;
        }
        if applies_to(car_id, &self.ids) {
            return true;
        }
        if self.read_at.is_some_and(|at| at.elapsed() < RECHECK) {
            return false;
        }
        self.ids = ids_in(&self.cars_dir);
        self.read_at = Some(Instant::now());
        applies_to(car_id, &self.ids)
    }
}

#[cfg(test)]
mod tests;
