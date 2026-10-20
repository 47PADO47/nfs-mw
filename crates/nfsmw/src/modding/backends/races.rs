//! `nfsmw.races` on the game side. Reads come from the world's last snapshot (`races_feed`); a start, an abort and the
//! two tunings are requests the world applies in its race update; a registered race is added to the list at once and
//! built by the world on its next frame. Race ids: the install's races are numbered by their place in the world's list,
//! a mod's from `CUSTOM_BASE`.

use blackbox_modding::abi::{BB_ERR_ARG, BB_ERR_NOT_FOUND};
use nfsmw_modding::abi::{BbName, BbRaceDef, BbRaceInfo, BbRaceResult, BbRaceState, BbVec3};
use nfsmw_modding::host::{BackendResult, RacesBackend};

use crate::modding::races_feed::{self, Custom, Request};

/// The `nfsmw.races` port of the game.
pub struct RacesPort;

/// The numbers of `RaceType` a race can have (`nfsmw_data::gameplay::RaceType`).
const KINDS: u32 = 11;
/// Most opponents a custom race takes: the Quick Race's largest choice.
const MAX_OPPONENTS: u32 = 7;

fn name_of(text: &str) -> BbName {
    BbName::new(text).unwrap_or(BbName::EMPTY)
}

fn vec3(p: [f32; 3]) -> BbVec3 {
    BbVec3 { x: p[0], y: p[1], z: p[2] }
}

/// Whether race `id` is on the list: a race with no row is not found.
fn known(id: u32) -> BackendResult<()> {
    match races_feed::with_snapshot(|s| s.rows.iter().any(|row| row.id == id)) {
        true => Ok(()),
        false => Err(BB_ERR_NOT_FOUND),
    }
}

impl RacesBackend for RacesPort {
    fn race_count(&self, _owner: &str) -> BackendResult<u32> {
        Ok(races_feed::with_snapshot(|s| s.rows.len() as u32))
    }

    fn race_at(&self, _owner: &str, index: u32) -> BackendResult<BbRaceInfo> {
        races_feed::with_snapshot(|s| {
            let row = s.rows.get(index as usize).ok_or(BB_ERR_NOT_FOUND)?;
            Ok(BbRaceInfo {
                size: 0,
                id: row.id,
                kind: row.kind,
                laps: row.laps,
                racers: row.racers,
                done: u8::from(row.done),
                _pad: [0; 3],
                pos: vec3(row.start),
                name: name_of(&row.name),
            })
        })
    }

    fn race_checkpoint_count(&self, _owner: &str, id: u32) -> BackendResult<u32> {
        races_feed::with_snapshot(|s| {
            s.rows.iter().find(|row| row.id == id).map(|row| row.checkpoints.len() as u32).ok_or(BB_ERR_NOT_FOUND)
        })
    }

    fn race_checkpoint_at(&self, _owner: &str, id: u32, index: u32) -> BackendResult<BbVec3> {
        races_feed::with_snapshot(|s| {
            let row = s.rows.iter().find(|row| row.id == id).ok_or(BB_ERR_NOT_FOUND)?;
            row.checkpoints.get(index as usize).map(|p| vec3(*p)).ok_or(BB_ERR_NOT_FOUND)
        })
    }

    fn race_start(&self, _owner: &str, id: u32) -> BackendResult<()> {
        known(id)?;
        races_feed::request(Request::Start(id));
        Ok(())
    }

    fn race_abort(&self, _owner: &str) -> BackendResult<()> {
        races_feed::request(Request::Abort);
        Ok(())
    }

    fn race_current(&self, _owner: &str) -> BackendResult<BbRaceState> {
        let now = races_feed::with_snapshot(|s| s.now);
        Ok(match now {
            Some(now) => BbRaceState {
                size: 0,
                active: 1,
                _pad: [0; 3],
                id: now.id,
                lap: now.lap,
                checkpoint: now.checkpoint,
                position: now.position,
                elapsed: now.elapsed,
            },
            None => BbRaceState {
                size: 0,
                active: 0,
                _pad: [0; 3],
                id: 0,
                lap: 0,
                checkpoint: 0,
                position: 0,
                elapsed: 0.0,
            },
        })
    }

    fn race_register(&self, owner: &str, def: &BbRaceDef, checkpoints: &[BbVec3]) -> BackendResult<u32> {
        let key = def.key.text().filter(|key| !key.is_empty()).ok_or(BB_ERR_ARG)?;
        // A mod's race is `modid:name`: a key under another mod's id would shadow its race.
        if !key.strip_prefix(owner).is_some_and(|rest| rest.starts_with(':')) {
            return Err(BB_ERR_ARG);
        }
        let name = def.name.text().filter(|name| !name.is_empty()).unwrap_or(key);
        let valid = def.kind < KINDS && def.laps >= 1 && def.opponents <= MAX_OPPONENTS && !checkpoints.is_empty();
        let finite = [def.start.x, def.start.y, def.start.z, def.start_heading]
            .into_iter()
            .chain(checkpoints.iter().flat_map(|p| [p.x, p.y, p.z]))
            .all(f32::is_finite);
        if !valid || !finite {
            return Err(BB_ERR_ARG);
        }
        let custom = Custom {
            id: 0,
            key: key.to_owned(),
            name: name.to_owned(),
            kind: def.kind,
            laps: def.laps,
            reward: def.reward_cash,
            opponents: def.opponents,
            start: [def.start.x, def.start.y, def.start.z],
            heading: def.start_heading,
            checkpoints: checkpoints.iter().map(|p| [p.x, p.y, p.z]).collect(),
        };
        races_feed::register(custom).ok_or(BB_ERR_ARG)
    }

    fn race_set_opponent_skill(&self, _owner: &str, id: u32, skill: f32) -> BackendResult<()> {
        known(id)?;
        if !(0.0..=1.0).contains(&skill) {
            return Err(BB_ERR_ARG);
        }
        races_feed::request(Request::Skill(id, skill));
        Ok(())
    }

    fn race_set_reward(&self, _owner: &str, id: u32, cash: u32) -> BackendResult<()> {
        known(id)?;
        races_feed::request(Request::Reward(id, cash));
        Ok(())
    }

    fn race_results(&self, _owner: &str, id: u32) -> BackendResult<BbRaceResult> {
        races_feed::with_snapshot(|s| {
            let (_, outcome) = s.results.iter().find(|(race, _)| *race == id).ok_or(BB_ERR_NOT_FOUND)?;
            Ok(BbRaceResult {
                size: 0,
                position: outcome.position,
                time: outcome.time,
                won: u8::from(outcome.won),
                _pad: [0; 3],
                winnings: outcome.winnings,
            })
        })
    }
}

#[cfg(test)]
#[path = "races_tests.rs"]
mod tests;
