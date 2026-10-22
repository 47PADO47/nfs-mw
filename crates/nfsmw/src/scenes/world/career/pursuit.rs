//! The end of a pursuit in career mode: bounty, heat, milestones and the evade autosave
//! (`docs/specs/career-economy-flows.md`, `docs/specs/career-milestones.md`, `docs/specs/ai-pursuit-heat.md` section 6).
//!
//! The world ends a chase with [`EndReason`]; there is no event for it, so [`WorldScene::career_pursuit`] watches the
//! chase from frame to frame. A chase that ends `Evaded` is recorded and the autosave runs; one that ends `Busted`
//! only reverts the milestones at stake and goes into the history (the bust itself is `bust.rs`).

use blackbox_pursuit::targets::{TargetId, Targets};
use blackbox_pursuit::{EndReason, Infraction, InfractionLog};
use nfsmw_career::coop::pursuit::settle_evade;
use nfsmw_career::economy::{InfractionKind, InfractionMask, PursuitResult, end_player_pursuit, end_pursuit_for};
use nfsmw_career::milestones::{Banked, StatType, pay_banked};
use nfsmw_career::phone::{BinMoment, bin_messages};
use nfsmw_career::rapsheet::{PursuitStats, case_id_text};
use nfsmw_career::safehouse::finish_cooldown_pursuit_default;
use nfsmw_career::safehouse::{
    DEFAULT_EVADE_HEAT_FACTOR, FrontEndContext, SPAWN_HEAT_FLOOR, apply_heat_factor, spawn_heat,
};
use nfsmw_career::savefile::Trigger;
use nfsmw_career::settings::CareerSettings;
use nfsmw_career::stable::Stable;

use super::rap_tally::ChaseTally;
use crate::career::{AutosaveInputs, handle_autosave};
use crate::scenes::world::WorldScene;
use crate::scenes::world::players::coop::{CoopSession, GUEST};
use crate::scenes::world::players::{LEAD, PlayerId};

/// The world's pursuits are free roam (play mode 0): milestones are tested while they run.
const FREE_ROAM: bool = true;
/// Ticks per second of the pursuit duration (statistic 0 is kept in 1/4000 s).
const TICKS_PER_SEC: f32 = 4000.0;

/// What the career saw of the pursuit last frame.
#[derive(Default)]
pub(in crate::scenes::world) struct PursuitWatch {
    /// A chase was on last frame: a chase starts when this turns true.
    pursuing: bool,
    /// The end reason last frame: a change to a finished reason is the end of a chase.
    end_reason: EndReason,
    /// Escapes in a row (the cop manager's counter): up on each evade, back to 0 on a bust.
    escapes_in_a_row: u32,
    /// Pursuits finished in this run; mixed into the case ids.
    finished: u32,
    /// The safe house unloaded the driven car's heat when it ended a chase (`note_safe_house_unload`); the evade
    /// factor leaves that car alone at the next end.
    safe_house_unloaded: bool,
    /// The counts only the world sees (cops involved, damaged, helicopters), for the rap sheet.
    tally: ChaseTally,
}

impl PursuitWatch {
    /// The safe house ended a cool-down chase and wrote the driven car's heat (`unload_world`). The next evade must
    /// not multiply that car again (`nfsmw_career::safehouse::heat`, the overwrite quirk).
    pub(in crate::scenes::world) fn note_safe_house_unload(&mut self) {
        self.safe_house_unloaded = true;
    }
}

/// The chase's numbers the career reads, taken from the world at each frame.
#[derive(Debug, Clone, Copy)]
struct Chase {
    /// Seconds of pursuit.
    secs: f32,
    /// Cop cars wrecked in the chase.
    cops_destroyed: u32,
    /// The infractions committed in the chase.
    infractions: InfractionLog,
    /// Normal rep points: the time in sight.
    rep_normal: u32,
    /// Rep points from cops destroyed.
    rep_destroyed: u32,
    /// The cost to state so far.
    cost_to_state: u32,
}

impl WorldScene {
    /// Banks the pursuit when it ends, and keeps the milestones and the car's heat in step with the chase.
    pub(super) fn career_pursuit(&mut self, _dt: f32) {
        let Some(traffic) = self.traffic.as_ref() else {
            return;
        };
        let pursuing = traffic.is_pursuing();
        let reason = traffic.end_reason();
        let chase = Chase {
            secs: traffic.pursuit_time(),
            cops_destroyed: traffic.cops_destroyed(),
            infractions: *traffic.infractions(),
            rep_normal: traffic.rep_normal_points(),
            rep_destroyed: traffic.rep_destroyed_points(),
            cost_to_state: traffic.cost_to_state(),
        };
        let started = pursuing && !self.career.pursuit.pursuing;
        let ended = (reason != EndReason::None && reason != self.career.pursuit.end_reason).then_some(reason);
        self.career.pursuit.pursuing = pursuing;
        self.career.pursuit.end_reason = reason;
        if started {
            self.career.handle.with(|career| career.milestones.start_pursuit());
            self.career.pursuit.tally.begin();
        }
        if pursuing {
            self.feed_running_stats(&chase);
            self.career.pursuit.tally.observe(traffic);
        }
        if let Some(reason) = ended {
            self.pursuit_ended(reason, &chase);
        }
        if !pursuing {
            self.load_carried_heat();
            self.load_career_bin();
        }
    }

    /// The statistics the world knows while a chase runs; the milestones at stake are tested against them.
    fn feed_running_stats(&self, chase: &Chase) {
        let kinds = chase.infractions.count() as f32;
        let bounty = chase.rep_normal.saturating_add(chase.rep_destroyed);
        self.career.handle.with(|career| {
            career.milestones.update_stat(StatType::PursuitLength, chase.secs, FREE_ROAM);
            career.milestones.update_stat(StatType::CopsDestroyedInPursuit, chase.cops_destroyed as f32, FREE_ROAM);
            career.milestones.update_stat(StatType::TotalInfractions, kinds, FREE_ROAM);
            career.milestones.update_stat(StatType::BountyInPursuit, bounty as f32, FREE_ROAM);
            career.milestones.update_stat(StatType::CostToStateInPursuit, chase.cost_to_state as f32, FREE_ROAM);
        });
    }

    /// The chase ended with `reason`: the record, the history, the milestones, the Blacklist and, on an evade, the
    /// autosave. The busted sequence (markers, cash, impound) is `bust.rs`.
    fn pursuit_ended(&mut self, reason: EndReason, chase: &Chase) {
        let busted = reason == EndReason::Busted;
        let escapes = match busted {
            true => 0,
            false => self.career.pursuit.escapes_in_a_row.saturating_add(1),
        };
        let seed = self.career.pursuit.finished ^ chase.secs.to_bits();
        let driven_kept = std::mem::take(&mut self.career.pursuit.safe_house_unloaded);
        let mask = infraction_mask(&chase.infractions);
        let result = pursuit_result(reason, mask, chase.rep_normal, chase.rep_destroyed);
        let roadblocks = self.traffic.as_ref().map(|traffic| traffic.roadblock_tally()).unwrap_or_default();
        let coop = self.coop.as_ref();
        let tally = &self.career.pursuit.tally;
        let phone = &mut self.career.phone;
        let mut rewards = None;
        let bins = self.career.handle.with(|career| {
            let unloaded = career.settings.car_handle;
            let (bank, others) = chase_banking(coop, unloaded);
            let kept = driven_kept.then(|| career.stable.career_for_handle(unloaded).map(|r| r.heat)).flatten();
            settle_chase_end(&career.settings, &mut career.stable, busted, bank, &others, &result, coop.is_some());
            restore_heat(&mut career.stable, unloaded, kept);
            let car_key = bank
                .and_then(|handle| career.stable.cars.iter().find(|car| car.handle == handle))
                .map_or(0, |car| car.frontend_key);
            let stats = pursuit_stats(
                car_key,
                chase.secs,
                chase.cops_destroyed,
                chase.infractions.count(),
                result.bounty(),
                chase.cost_to_state,
            );
            let mut stats = stats;
            tally.write(roadblocks, &mut stats);
            career.history.record(&stats, busted, case_id(seed));
            let milestones = career.milestones.on_pursuit_end(&result, FREE_ROAM, chase.secs, escapes);
            if let Some(record) = bank.and_then(|handle| career.stable.career_for_handle_mut(handle)) {
                pay_banked(record, &milestones.banked);
            }
            let bounty =
                bank.and_then(|handle| career.stable.career_for_handle(handle)).map_or(0, |record| record.bounty);
            rewards = super::rewards::report_of(&career.milestones, &milestones.banked, bounty);
            // A banked milestone is a challenge passed in its bin: the Blacklist counts it, and the bin's challenge
            // messages follow its count (`docs/specs/career-phone.md` section 7).
            for (bin, key) in passed_challenges(&career.milestones, &milestones.banked) {
                career.blacklist.challenge_passed(bin, &key);
                let count = career.blacklist.bin(bin).map_or(0, |record| record.challenges.len());
                let count = u8::try_from(count).unwrap_or(u8::MAX);
                phone.queue_bin_messages(&mut career.settings.sms, bin_messages(bin, BinMoment::Challenges(count)));
            }
            let total = u32::try_from(career.stable.total_bounty()).unwrap_or(u32::MAX);
            career.blacklist.bounty_changed(total)
        });
        if let Some(bins) = bins
            && !bins.is_empty()
        {
            bevy_log::info!("the bounty completed the bins {bins:?}");
        }
        if let Some(report) = rewards {
            self.career.rewards = Some(report);
        }
        self.career.pursuit.escapes_in_a_row = escapes;
        self.career.pursuit.finished = self.career.pursuit.finished.saturating_add(1);
        if !busted {
            self.autosave_pursuit_evaded();
        }
    }

    /// Asks the autosave for the "pursuit evaded" trigger. Free roam: no event is underway and no node is current.
    fn autosave_pursuit_evaded(&self) {
        let inputs = AutosaveInputs {
            context: FrontEndContext::CAREER,
            node_id: None,
            boss_race: false,
            gameplay_idle: true,
            profile_loaded: true,
        };
        handle_autosave(
            &self.career.handle,
            self.career.profiles.as_ref(),
            Trigger::PursuitEvaded { event_underway: false },
            inputs,
        );
    }

    /// Tells the police the Blacklist's current bin (or that no career is on): the bin sets the heat range and how fast a
    /// chase heats up, read when the next chase starts.
    fn load_career_bin(&mut self) {
        let bin = self.career.handle.with(|career| career.blacklist.current_bin());
        if let Some(traffic) = self.traffic.as_mut() {
            traffic.set_career_bin(bin);
        }
    }

    /// Loads the current car's heat into the police, so the next chase starts from it. Runs each frame outside a
    /// chase: a chase's `begin` reads the value the frame before it starts.
    fn load_carried_heat(&mut self) {
        let guest_handle = self.coop.as_ref().and_then(|session| session.guest_handle);
        let heat = self.career.handle.with(|career| {
            let lead = spawn_heat(&career.settings, &career.stable, true);
            let guest = guest_handle
                .and_then(|handle| career.stable.career_for_handle(handle))
                .map(|record| record.heat.max(SPAWN_HEAT_FLOOR));
            carried(lead, guest)
        });
        let Some(heat) = heat else {
            return;
        };
        let Some(traffic) = self.traffic.as_mut() else {
            return;
        };
        traffic.set_carried_heat(heat);
    }
}

/// The bin and the challenge name of each banked milestone, in the order they were banked. A key the set does not know is
/// skipped.
fn passed_challenges(set: &nfsmw_career::milestones::MilestoneSet, banked: &[Banked]) -> Vec<(u8, String)> {
    banked
        .iter()
        .filter_map(|b| set.milestones().iter().find(|m| m.def.key == b.key))
        .map(|m| (m.def.bin, m.def.key.to_string()))
        .collect()
}

/// The rule crate's result of a chase that ended `reason` with `infractions`, banking the world's rep points: the
/// normal ones (time in sight) and the destruction ones (`docs/specs/ai-pursuit-heat.md` section 5.1).
pub(super) fn pursuit_result(
    reason: EndReason,
    infractions: InfractionMask,
    rep_normal: u32,
    rep_destroyed: u32,
) -> PursuitResult {
    PursuitResult {
        busted: reason == EndReason::Busted,
        infractions,
        rep_points_normal: rep_normal,
        rep_points_destroyed: rep_destroyed,
        ..PursuitResult::default()
    }
}

/// The infraction kinds of a chase's log as the rule crate's mask.
pub(super) fn infraction_mask(log: &InfractionLog) -> InfractionMask {
    let mut mask = InfractionMask::NONE;
    for infraction in log.iter() {
        mask.insert(kind_of(infraction));
    }
    mask
}

/// The rule crate's kind of a world infraction (same order, same bit).
fn kind_of(infraction: Infraction) -> InfractionKind {
    match infraction {
        Infraction::Speeding => InfractionKind::Speeding,
        Infraction::Racing => InfractionKind::Racing,
        Infraction::RecklessDriving => InfractionKind::RecklessDriving,
        Infraction::Assault => InfractionKind::Assault,
        Infraction::HitAndRun => InfractionKind::HitAndRun,
        Infraction::DamageToProperty => InfractionKind::DamageToProperty,
        Infraction::ResistingArrest => InfractionKind::ResistingArrest,
        Infraction::OffRoad => InfractionKind::OffRoad,
    }
}

/// The history's numbers of a chase. The duration, the cops wrecked, the infraction kinds, the bounty and the cost to
/// state are known to the world; the rest are 0 until the world counts them.
pub(super) fn pursuit_stats(
    car_key: u32,
    secs: f32,
    cops_destroyed: u32,
    infraction_kinds: u32,
    bounty: u32,
    cost_to_state: u32,
) -> PursuitStats {
    PursuitStats {
        car_key,
        bounty,
        ticks: ticks_of(secs),
        cops_destroyed,
        infraction_kinds,
        cost_to_state,
        ..PursuitStats::default()
    }
}

/// A duration in seconds as statistic ticks (1/4000 s). A negative or NaN duration is 0.
pub(super) fn ticks_of(secs: f32) -> u32 {
    (secs.max(0.0) * TICKS_PER_SEC).round() as u32
}

/// The case id of a pursuit: eight characters, a digit then a capital letter in turn, drawn from `seed` with an
/// xorshift. The original's generator is not known; the same seed gives the same id.
pub(super) fn case_id(seed: u32) -> [u8; 12] {
    let mut state = seed | 1;
    let mut chars = [0u8; 8];
    for (i, c) in chars.iter_mut().enumerate() {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        *c = match i % 2 {
            0 => b'0' + (state % 10) as u8,
            _ => b'A' + (state % 26) as u8,
        };
    }
    case_id_text(&chars)
}

/// The car that banks the chase that just ended, and the other cars it still held (`bank_handles`). A single player's
/// chase banks on the current car and has no others.
fn chase_banking(coop: Option<&CoopSession>, lead: u32) -> (Option<u32>, Vec<u32>) {
    let Some(session) = coop else {
        return (Some(lead), Vec::new());
    };
    bank_handles(lead, session.guest_handle, &session.chase.last_targets, session.chase.last_primary)
}

/// The car of a co-op player: the lead's is the current car, the guest's is `guest`; `None` for any other player.
fn handle_of(id: PlayerId, lead: u32, guest: Option<u32>) -> Option<u32> {
    if id == LEAD {
        return Some(lead);
    }
    if id == GUEST {
        return guest;
    }
    None
}

/// The car that banks a co-op chase, and the other cars still chased in it when it ended. The chase banks on the car
/// of its last primary: the lead's car, or the guest's (`None` when the guest has no career car, so nothing banks).
/// The others are the other chased players' cars; a player with no career car is not listed [ours].
pub(super) fn bank_handles(
    lead: u32,
    guest: Option<u32>,
    targets: &Targets,
    primary: PlayerId,
) -> (Option<u32>, Vec<u32>) {
    let bank = handle_of(primary, lead, guest);
    let others = targets
        .chased()
        .map(|id: TargetId| PlayerId(id.0))
        .filter(|&id| id != primary)
        .filter_map(|id| handle_of(id, lead, guest))
        .collect();
    (bank, others)
}

/// Writes the end of a chase into the stable. A single player's chase runs as it always did, on the current car. A
/// co-op chase banks on `bank`: a bust marks that car; an evade runs the heat factor once, then `settle_evade` for the
/// banking car and the other cars of the chase [ours].
fn settle_chase_end(
    settings: &CareerSettings,
    stable: &mut Stable,
    busted: bool,
    bank: Option<u32>,
    others: &[u32],
    result: &PursuitResult,
    coop: bool,
) {
    if !coop {
        match busted {
            true => end_player_pursuit(settings, stable, result),
            false => finish_cooldown_pursuit_default(settings, stable, result),
        };
        return;
    }
    if busted {
        if let Some(bank) = bank {
            end_pursuit_for(bank, stable, result);
        }
        return;
    }
    apply_heat_factor(stable, DEFAULT_EVADE_HEAT_FACTOR);
    settle_evade(stable, bank, others, result);
}

/// The heat a chase starts from: the lead car's heat, or the higher of it and the guest car's when the guest has a
/// record [ours].
fn carried(lead: f32, guest: Option<f32>) -> f32 {
    guest.map_or(lead, |guest| lead.max(guest))
}

/// Writes `heat` back into the record of `handle`, when there is one and a heat was kept.
fn restore_heat(stable: &mut Stable, handle: u32, heat: Option<f32>) {
    let (Some(heat), Some(record)) = (heat, stable.career_for_handle_mut(handle)) else {
        return;
    };
    record.heat = heat;
}

#[cfg(test)]
#[path = "pursuit_tests.rs"]
mod tests;
