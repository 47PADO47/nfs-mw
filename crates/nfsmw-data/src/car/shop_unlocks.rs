//! Installed career-bin garage awards. See `docs/specs/garage-shop.md`.

use std::sync::Arc;

use anyhow::{Context, Result, ensure};
use blackbox_attrib::{Database, Value};
use game_install::GameDir;

const PART_ID: u32 = 0x3c2f_daab;
const LEVEL: u32 = 0xd267_facc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Unlock {
    /// Awarded when entering this installed Blacklist bin.
    pub bin: u8,
    pub entity: u32,
    pub level: i8,
}

pub fn load(dir: &GameDir) -> Result<Arc<Vec<Unlock>>> {
    crate::memo(dir, &[crate::shared::ATTRIBUTES_FILE, crate::gameplay::GAMEPLAY_FILE], || {
        let attributes = crate::shared::attributes(dir)?;
        let mut db = Database::clone(&attributes);
        db.load(&crate::read_unwrapped(dir, crate::gameplay::GAMEPLAY_FILE)?)?;
        read(&db)
    })
}

fn read(db: &Database) -> Result<Vec<Unlock>> {
    let mut result = Vec::new();
    for bin in crate::gameplay::bins(db) {
        let Ok(number @ 1..=16) = u8::try_from(bin.number) else { continue };
        for key in bin.unlocks {
            let Some(collection) = crate::gameplay::collection(db, key) else { continue };
            let Some(part) = collection.get_by_key(PART_ID).and_then(Value::as_str) else { continue };
            if award(part, 0)?.is_none() {
                continue;
            }
            let Some(level) = collection.get_by_key(LEVEL).and_then(Value::as_i32) else {
                anyhow::bail!("Garage unlock award {} has no level", collection.key());
            };
            if let Some((entity, level)) = award(part, level)? {
                result.push(Unlock { bin: number, entity, level });
            }
        }
    }
    ensure!(!result.is_empty(), "Installed garage unlock awards are unavailable");
    Ok(result)
}

fn award(part: &str, level: i32) -> Result<Option<(u32, i8)>> {
    let entity = match part.trim() {
        "tires" => 4,
        "brakes" => 5,
        "chassis" => 6,
        "transmission" => 7,
        "engine" => 8,
        "induction" => 9,
        "nos" => 10,
        "bodykit" => 11,
        "spoiler" => 12,
        "rims" => 13,
        "hood" => 14,
        "roofscoop" => 15,
        "hud" => 17,
        "tint" => 18,
        "paint" => 23,
        "vinyls" => 40,
        "numbers" => 43,
        "decals" => match level {
            1 => 44,
            2 => 46,
            3 => 48,
            _ => 50,
        },
        _ => return Ok(None),
    };
    let level = i8::try_from(level.max(0)).context("Installed garage unlock level is out of range")?;
    Ok(Some((entity, level)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn awards_retain_levels_and_use_saved_unlock_entity_order() {
        assert_eq!(award("engine", 3).unwrap(), Some((8, 3)));
        assert_eq!(award("bodykit", 4).unwrap(), Some((11, 4)));
        assert_eq!(award("paint", -1).unwrap(), Some((23, 0)));
        assert_eq!(award("mod_only", 1).unwrap(), None);
        assert!(award("engine", 128).is_err());
    }

    #[test]
    fn decal_awards_open_each_original_zone_with_their_original_level() {
        assert_eq!(award("decals", 1).unwrap(), Some((44, 1)));
        assert_eq!(award("decals", 2).unwrap(), Some((46, 2)));
        assert_eq!(award("decals", 3).unwrap(), Some((48, 3)));
    }

    #[test]
    #[ignore = "needs the installed game"]
    fn installed_awards_cover_performance_and_visual_categories() {
        let folder = std::env::var_os("NFSMW_GAME_DIR").expect("NFSMW_GAME_DIR must select an install");
        let dir = GameDir::open(folder).unwrap();
        let awards = load(&dir).unwrap();
        for entity in (4..=15).chain([17, 18, 23, 40, 43, 44, 46, 48]) {
            assert!(awards.iter().any(|award| award.entity == entity), "missing category {entity}");
        }
        assert!(awards.iter().all(|award| (1..=16).contains(&award.bin)));
    }
}
