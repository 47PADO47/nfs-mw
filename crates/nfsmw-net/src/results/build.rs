//! Joining a mode's placings, the session tally and the roster into a [`RaceRecord`].

use nfsmw_sim::Tick;

use crate::message::{List, Mode};

use super::placing::Placing;
use super::record::{RaceRecord, RecordRow};
use super::roster::Roster;
use super::tally::Tally;

/// What a record says about the race itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecordMeta {
    pub mode: Mode,
    pub track: u32,
    pub laps: u8,
    pub start_tick: Tick,
    pub end_tick: Tick,
}

/// The record of a race: one row per placing whose player has a seat in `roster`, in the placings' order, with the
/// tally's stats (zero for a player the tally never saw). An end tick before the start is clamped to the start.
pub fn build_record(meta: RecordMeta, placings: &[Placing], tally: &Tally, roster: &Roster) -> RaceRecord {
    let mut rows = List::empty();
    for placing in placings {
        let seat = match roster.get(placing.player) {
            Some(seat) => seat,
            None => continue,
        };
        let stats = tally.get(placing.player).copied().unwrap_or_default();
        let row = RecordRow {
            slot: seat.slot,
            entity_id: seat.entity,
            place: placing.place,
            flags: placing.flags,
            time_ms: placing.time_ms.unwrap_or(0),
            best_lap_ms: placing.best_lap_ms.unwrap_or(0),
            top_speed_cms: centimetres(stats.top_speed),
            damage: unit_byte(stats.damage),
            busted: stats.busted,
            pursuit_ms: stats.pursuit_ms(),
            resets: stats.resets,
            points: finite_or_zero(placing.points),
        };
        if rows.push(row).is_err() {
            break;
        }
    }
    RaceRecord {
        mode: meta.mode,
        track: meta.track,
        laps: meta.laps,
        start_tick: meta.start_tick,
        end_tick: meta.end_tick.max(meta.start_tick),
        rows,
    }
}

/// A speed in m/s as whole cm/s, 0 for a negative or non-finite speed.
fn centimetres(speed: f32) -> u32 {
    let cms = (finite_or_zero(speed) * 100.0).round();
    if cms <= 0.0 {
        return 0;
    }
    cms.min(u32::MAX as f32) as u32
}

/// A 0..1 value as 0..=255.
fn unit_byte(value: f32) -> u8 {
    (finite_or_zero(value).clamp(0.0, 1.0) * 255.0).round() as u8
}

fn finite_or_zero(value: f32) -> f32 {
    match value.is_finite() {
        true => value,
        false => 0.0,
    }
}
