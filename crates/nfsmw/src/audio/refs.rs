//! Which bank sound each effect of the car sound plays. The banks come from the car's sound set and the
//! game-wide lists; the sound numbers inside them are listed in `docs/formats/audio.md` ("Which bank sound").
//! Numbers are `BNKl` entries, counted from 1.

use blackbox_carsound::{ScrapeKind, SoundRef};
use nfsmw_data::sound::{CarSound, ENGINE_DIR, NOS_DIR, SHIFTING_DIR, SKIDS_DIR, TURBO_DIR};

use super::Group;

/// Bank of the road noise loops, the wind and the scrape loops, under `SOUND/`.
pub const ROAD_NOISE_BANK: &str = "IG_GLOBAL/ROADNOISE_00_MB.abk";
pub const WIND_BANK: &str = "IG_GLOBAL/WIND_00_MB.abk";
pub const SCRAPE_BANK: &str = "IG_GLOBAL/FX_MAIN_MEM_MB.abk";
pub const COLLISION_BANK: &str = "IG_GLOBAL/Stich_Collision_MB.abk";

/// A sound in a bank.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BankRef {
    /// Path under `SOUND/`.
    pub bank: String,
    pub index: usize,
    pub group: Group,
}

fn at(dir: &str, bank: &str, index: usize, group: Group) -> Option<BankRef> {
    let dir = dir.strip_prefix("SOUND/").unwrap_or(dir);
    (!bank.is_empty()).then(|| BankRef { bank: format!("{dir}/{bank}"), index, group })
}

/// The sound `sound` plays for this car, if its banks have one.
pub fn resolve(car: &CarSound, sound: SoundRef) -> Option<BankRef> {
    let engine = &car.engine;
    match sound {
        SoundRef::GearClunk { up } => at(SHIFTING_DIR, &car.shift.bank, if up { 1 } else { 2 }, Group::Sfx),
        SoundRef::BrakeMash => at(SHIFTING_DIR, &car.shift.bank, 3, Group::Sfx),
        // The sweetener bank holds short engine noises; 0 is the longer "pssh" of letting go, 1 the shorter
        // one of taking up (unconfirmed, see the doc).
        SoundRef::Sweetener(kind) => {
            at(ENGINE_DIR, engine.sweet_banks.first()?, if kind.sample() == 0 { 10 } else { 8 }, Group::Engine)
        }
        SoundRef::ReverseWhine => at(ENGINE_DIR, engine.sweet_banks.get(1)?, 1, Group::Engine),
        SoundRef::TurboSpool => at(TURBO_DIR, &car.turbo.as_ref()?.bank, 1, Group::Engine),
        SoundRef::TurboBlowoff(n) => at(TURBO_DIR, &car.turbo.as_ref()?.bank, 2 + usize::from(n.min(2)), Group::Sfx),
        SoundRef::Nitrous => at(NOS_DIR, car.banks.nitrous_bank()?, 1, Group::Sfx),
        SoundRef::Purge => at(NOS_DIR, car.banks.nitrous_bank()?, 3, Group::Sfx),
        SoundRef::Skid { surface, sideways } => {
            // Asphalt and the like have a squeal and a burnout loop, the loose surfaces (dirt, grass) another pair.
            let pair = usize::from(surface.min(1)) * 2;
            at(SKIDS_DIR, car.banks.skid_bank()?, 1 + pair + usize::from(!sideways), Group::Sfx)
        }
        // The loops are sounds 1 to 7 for the loop values 0 to 6; metal and the stitch loop have none.
        SoundRef::RoadNoise(n) => {
            (n <= 6).then(|| BankRef { bank: ROAD_NOISE_BANK.into(), index: usize::from(n) + 1, group: Group::Sfx })
        }
        SoundRef::Wind => Some(BankRef { bank: WIND_BANK.into(), index: 1, group: Group::Sfx }),
        SoundRef::Scrape(kind) => {
            let index = match kind {
                ScrapeKind::Ground => 1,
                ScrapeKind::Wall => 2,
                ScrapeKind::Car => 3,
            };
            Some(BankRef { bank: SCRAPE_BANK.into(), index, group: Group::Sfx })
        }
    }
}
