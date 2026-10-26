//! Which banks a car needs, handed to the decode worker as soon as the car's sound set is loaded so its first sounds are ready
//! when the frame asks for them (see `decode_worker`).

use nfsmw_data::sound::{CarSound, ENGINE_DIR, NOS_DIR, SHIFTING_DIR, SKIDS_DIR, SoundUpgrades, TURBO_DIR};

use super::Audio;
use super::decode_worker::Preload;
use super::refs::COLLISION_BANK;

/// The bank of the transmission whine, played by every car.
const TRANSMISSION_BANK: &str = "ENGINE/CAR_TRANNY.abk";

/// `name` in `dir` as a path under `SOUND/`, `None` for an empty name.
fn under(dir: &str, name: &str) -> Option<String> {
    let dir = dir.strip_prefix("SOUND/").unwrap_or(dir);
    (!name.is_empty()).then(|| format!("{dir}/{name}"))
}

/// The banks of the car's own sounds (small, so every sound of them is decoded) in the order they are first needed.
fn car_banks(set: &CarSound) -> Vec<String> {
    let engine = &set.engine;
    let mut banks = vec![under(ENGINE_DIR, &engine.bank_main)];
    banks.extend(engine.sweet_banks.iter().map(|name| under(ENGINE_DIR, name)));
    banks.push(under(SHIFTING_DIR, &set.shift.bank));
    banks.push(set.turbo.as_ref().and_then(|turbo| under(TURBO_DIR, &turbo.bank)));
    banks.push(set.banks.nitrous_bank().and_then(|name| under(NOS_DIR, name)));
    banks.push(set.banks.skid_bank().and_then(|name| under(SKIDS_DIR, name)));
    banks.into_iter().flatten().collect()
}

impl Audio {
    /// Starts decoding the sound set of the car the scene reports, on the frame it first reports it (or changes its
    /// performance levels), before the engine's own load: the worker decodes while the frame prepares the rest. Nothing
    /// for a car that is playing already, one that failed, or without a sound device.
    pub(super) fn preload_car_sounds(&mut self, car: &str, upgrades: SoundUpgrades) {
        if self.car.as_ref().is_some_and(|playing| playing.car == car && playing.upgrades == upgrades) {
            return;
        }
        if !self.available() {
            return;
        }
        if self.failed.as_ref().is_some_and(|(failed, levels)| failed == car && *levels == upgrades) {
            return;
        }
        let Ok(set) = self.sound_set(car, upgrades) else { return };
        self.preload_sound_set(&set);
    }

    /// Starts decoding the banks of a car's sound set, then the game-wide banks its effects use.
    pub(super) fn preload_sound_set(&mut self, set: &CarSound) {
        if !self.available() {
            return;
        }
        for bank in car_banks(set) {
            self.decoder.preload(&self.dir, &bank, Preload::All);
        }
        self.decoder.preload(&self.dir, TRANSMISSION_BANK, Preload::Tables);
        self.decoder.preload(&self.dir, COLLISION_BANK, Preload::Tables);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_path_is_under_sound_and_an_empty_name_has_none() {
        assert_eq!(under(ENGINE_DIR, "CAR_66.abk").as_deref(), Some("ENGINE/CAR_66.abk"));
        assert_eq!(under(ENGINE_DIR, ""), None);
    }
}
