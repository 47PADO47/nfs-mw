//! The career state in the career-flow trace (`parity/career_trace.rs`): a `career_state` line whenever the state the
//! story flow moves on changes: the stage (bin), the cash, the cars owned and the current car, the settings flags, the
//! nodes won and unlocked in the career, the rival beaten, and whether a career is played at all. It is read once a
//! frame, and only while a trace is recorded.

use std::sync::Mutex;

use nfsmw_career::carlot;
use nfsmw_career::settings::{NodeState, node_hash};
use serde_json::json;

use super::Frontend;
use crate::career::{Career, race_event_id};

/// What a `career_state` line is compared on.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Key {
    playing: bool,
    alias: String,
    stage: u8,
    cash: u32,
    car_handle: u32,
    owned: usize,
    flags: u32,
    won: Vec<u32>,
    unlocked: usize,
    rival_beaten: Option<u8>,
}

static LAST: Mutex<Option<Key>> = Mutex::new(None);

fn key_of(career: &Career, playing: bool) -> Key {
    let nodes = career.settings.nodes();
    let won = nodes.records().iter().filter(|r| r.state.won_in_career()).map(|r| r.hash).collect();
    let unlocked = nodes.records().iter().filter(|r| r.state.flags & NodeState::UNLOCKED_IN_CAREER != 0).count();
    Key {
        playing,
        alias: career.alias.clone(),
        stage: career.stage(),
        cash: career.settings.cash,
        car_handle: career.settings.car_handle,
        owned: carlot::owned_count(&career.stable),
        flags: career.settings.flags.0,
        won,
        unlocked,
        rival_beaten: career.rival_beaten,
    }
}

/// The event ids of the node keys `hashes`, as the career's races name them; a key no race has stays a hex number.
fn event_ids(career: &Career, hashes: &[u32]) -> Vec<String> {
    hashes
        .iter()
        .map(|hash| {
            career
                .data
                .gameplay
                .races
                .iter()
                .map(race_event_id)
                .find(|event| node_hash(event) == *hash)
                .unwrap_or_else(|| format!("{hash:#010x}"))
        })
        .collect()
}

impl Frontend {
    /// Writes a `career_state` line when the career's state differs from the last line's.
    pub(in crate::frontend) fn trace_career_state(&self) {
        if !crate::parity::career_trace::enabled() {
            return;
        }
        let playing = self.career.is_active();
        let Some(key) = self.career.with_loaded(|career| key_of(career, playing)) else { return };
        let Ok(mut last) = LAST.lock() else { return };
        if last.as_ref() == Some(&key) {
            return;
        }
        let won = self.career.with_loaded(|career| event_ids(career, &key.won)).unwrap_or_default();
        let line = json!({
            "playing": key.playing, "alias": key.alias, "stage": key.stage, "cash": key.cash,
            "car_handle": key.car_handle, "car": self.profile_car(), "cars_owned": key.owned,
            "flags": format!("{:#010x}", key.flags), "won_in_career": won, "unlocked_in_career": key.unlocked,
            "rival_beaten": key.rival_beaten,
        });
        *last = Some(key);
        drop(last);
        crate::parity::career_trace::record("career_state", line);
    }
}
