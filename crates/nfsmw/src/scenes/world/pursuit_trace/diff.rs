//! The events of a pursuit trace, found by comparing one police snapshot with the one before. A car that comes and
//! goes between two samples (50 ms) is not seen. Removal reasons are inferred from the last snapshot the car was in.
use std::collections::{BTreeMap, BTreeSet};

use serde_json::{Value, json};

use super::records::{cop_id, in_pursuit};
use crate::scenes::world::ai::trace_frame::{CopFrame, PoliceFrame};

/// A cop unseen this long (seconds) was most likely retired for being out of sight.
const OFFSCREEN_RETIRE: f32 = 3.0;

/// One event found: its kind, the cop it is about (in the snapshot it was last or first seen in) and the remake-only
/// details.
#[derive(Clone, Debug)]
pub(super) struct Found {
    pub kind: &'static str,
    pub cop: Option<CopFrame>,
    /// The snapshot the cop is described with was a chase member (for `pursuit_leave` / `unspawn`, the earlier one).
    pub member: bool,
    pub detail: Value,
}

fn found(kind: &'static str, detail: Value) -> Found {
    Found { kind, cop: None, member: false, detail }
}

fn about(kind: &'static str, cop: &CopFrame, member: bool, detail: Value) -> Found {
    Found { kind, cop: Some(cop.clone()), member, detail }
}

fn distance(a: [f32; 3], b: [f32; 3]) -> f32 {
    a.iter().zip(b).map(|(x, y)| (x - y) * (x - y)).sum::<f32>().sqrt()
}

/// The previous snapshot and the number of the chase in progress.
#[derive(Default)]
pub(super) struct Tracker {
    before: Option<(PoliceFrame, Option<[f32; 3]>)>,
    /// Chases started since the recording began; the running chase is the last.
    pub pursuits: u32,
}

impl Tracker {
    /// The id of the running chase, as the cops' `pursuit` field gives it.
    pub fn pursuit_id(&self, frame: &PoliceFrame) -> Option<String> {
        frame.pursuing.then(|| format!("remake-pursuit-{}", self.pursuits))
    }

    /// The events between the last snapshot and `now`. `player` is the player's physics position.
    pub fn events(&mut self, now: &PoliceFrame, player: Option<[f32; 3]>) -> Vec<Found> {
        let empty = PoliceFrame::default();
        let (before, before_player) = match &self.before {
            Some((frame, at)) => (frame, *at),
            None => (&empty, None),
        };
        let mut out = Vec::new();
        if !before.pursuing && now.pursuing {
            self.pursuits += 1;
            out.push(found("pursuit_start", json!({"heat_level": now.heat_level, "heat": now.heat})));
        }
        let created = roadblock_changes(before, now, &mut out);
        cop_changes(before, now, before_player, created, &mut out);
        chase_changes(before, now, &mut out);
        for event in &now.speech {
            out.push(found("speech_event", json!({"event": event})));
            if event.starts_with("BackupRequested") {
                out.push(found("backup_request", json!({"event": event})));
            }
            if event.starts_with("BackupDenied") {
                out.push(found("backup_denied", json!({"event": event})));
            }
        }
        if now.speech_lost > 0 {
            out.push(found("speech_events_lost", json!({"count": now.speech_lost})));
        }
        self.before = Some((now.clone(), player));
        out
    }
}

/// Spawns, joins, leaves and removals of the cops.
fn cop_changes(before: &PoliceFrame, now: &PoliceFrame, player: Option<[f32; 3]>, created: bool, out: &mut Vec<Found>) {
    let old: BTreeMap<u32, &CopFrame> = before.cops.iter().map(|c| (c.key, c)).collect();
    let new: BTreeMap<u32, &CopFrame> = now.cops.iter().map(|c| (c.key, c)).collect();
    for (key, cop) in &new {
        let member = in_pursuit(cop, now);
        let Some(previous) = old.get(key) else {
            out.push(about("spawn", cop, member, json!({"kind": cop.kind, "model": cop.model})));
            if member {
                out.push(about("pursuit_join", cop, true, Value::Null));
            }
            continue;
        };
        let was = in_pursuit(previous, before);
        if !was && member {
            out.push(about("pursuit_join", cop, true, Value::Null));
        }
        if was && !member {
            out.push(about("pursuit_leave", cop, false, Value::Null));
        }
    }
    for (key, cop) in &old {
        if new.contains_key(key) {
            continue;
        }
        let was = in_pursuit(cop, before);
        if was {
            out.push(about("pursuit_leave", cop, true, Value::Null));
        }
        let reason = retire_reason(cop, before, now, created);
        let gap = player.map(|at| distance(cop.position, at));
        let detail = json!({"reason": reason, "reason_inferred": true, "distance_to_player_m": gap,
            "offscreen_s": cop.offscreen_secs, "kind": cop.kind});
        out.push(about("unspawn", cop, was, detail));
    }
}

/// Why a cop that is gone was most likely removed, from the last snapshot it was in.
pub(super) fn retire_reason(cop: &CopFrame, before: &PoliceFrame, now: &PoliceFrame, created: bool) -> &'static str {
    if before.pursuing && !now.pursuing {
        return "pursuit_end";
    }
    if now.lockout_secs > 0.0 && before.lockout_secs <= 0.0 {
        return "lockout";
    }
    if created && cop.roadblock.is_none() {
        return "roadblock_takeover";
    }
    if cop.wrecked {
        return "wrecked";
    }
    if cop.fleeing {
        return "fled";
    }
    if cop.offscreen_secs >= OFFSCREEN_RETIRE {
        return "offscreen";
    }
    "unknown"
}

/// Roadblocks put up and taken down, with each car and spike strip. True when one went up.
fn roadblock_changes(before: &PoliceFrame, now: &PoliceFrame, out: &mut Vec<Found>) -> bool {
    let ids = |frame: &PoliceFrame| -> BTreeSet<u32> {
        frame.cops.iter().filter_map(|c| c.roadblock).chain(frame.strips.iter().map(|s| s.roadblock)).collect()
    };
    let (old, new) = (ids(before), ids(now));
    let mut created = false;
    for id in new.difference(&old) {
        created = true;
        out.push(found("roadblock_create", roadblock(*id, now)));
    }
    for id in old.difference(&new) {
        out.push(found("roadblock_remove", roadblock(*id, before)));
    }
    created
}

fn roadblock(id: u32, frame: &PoliceFrame) -> Value {
    let cars: Vec<Value> = frame
        .cops
        .iter()
        .filter(|c| c.roadblock == Some(id))
        .map(|c| json!({"id": cop_id(c.key), "model": c.model, "position": c.position, "forward": c.forward}))
        .collect();
    let strips: Vec<Value> = frame
        .strips
        .iter()
        .filter(|s| s.roadblock == id)
        .map(|s| json!({"index": s.index, "position": s.position, "axis": s.axis}))
        .collect();
    json!({"roadblock": id, "cars": cars, "spike_strips": strips})
}

/// The chase's own transitions: heat, phase, cool-down, support, the helicopter and the end.
fn chase_changes(before: &PoliceFrame, now: &PoliceFrame, out: &mut Vec<Found>) {
    if before.pursuing && now.pursuing && before.heat_level != now.heat_level {
        out.push(found("heat_change", json!({"from": before.heat_level, "to": now.heat_level, "heat": now.heat})));
    }
    if before.phase != now.phase {
        out.push(found("phase_change", json!({"from": before.phase, "to": now.phase})));
    }
    if before.cooldown != now.cooldown {
        out.push(found("cooldown_change", json!({"from": before.cooldown, "to": now.cooldown})));
    }
    if before.support_order != now.support_order && now.support_order.is_some() {
        out.push(found("support_request", json!({"order": now.support_order, "previous": before.support_order})));
    }
    if before.heli.is_none()
        && let Some(heli) = &now.heli
    {
        out.push(found("heli_spawn", json!({"position": heli.position, "forward": heli.forward, "mode": heli.mode})));
    }
    if now.heli.is_none()
        && let Some(heli) = &before.heli
    {
        out.push(found("heli_despawn", json!({"position": heli.position, "mode": heli.mode})));
    }
    if before.pursuing && !now.pursuing {
        let reason = now.end_reason.as_str();
        if reason == "Evaded" {
            out.push(found("pursuit_evaded", json!({"reason": reason})));
        }
        if reason == "Busted" {
            out.push(found("busted", json!({"reason": reason})));
        }
        out.push(found("pursuit_end", json!({"reason": reason, "heat": before.heat})));
    }
}

#[cfg(test)]
#[path = "diff_tests.rs"]
mod tests;
