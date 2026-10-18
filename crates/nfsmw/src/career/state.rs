//! The [`Career`] being played: the profile file with its rule state (settings, car stable, pursuit history, Blacklist,
//! milestones, speed traps) and the autosave gate. Spec index: `docs/specs/career.md`.
//!
//! The rule state lives in `nfsmw_career` types. [`Career::profile_bytes`] writes it back over the profile file kept
//! from the load, so everything the rules do not read (options, playlist, statistics, controller bindings, unknown
//! bytes) comes out as it went in.
//!
//! The milestones and speed traps are stored in the milestone block, which is the first 0x4000 bytes of the settings'
//! gameplay blob. A profile whose block cannot be laid out (a blank one has zero header counts) starts with fresh
//! milestones and traps built from the data; see [`fresh_progress`].

use std::sync::Arc;

use anyhow::{Context, Result};
use nfsmw_career::blacklist::Blacklist;
use nfsmw_career::economy::markers::MarkerInventory;
use nfsmw_career::milestones::speedtrap::SpeedTrap;
use nfsmw_career::milestones::{Milestone, MilestoneSet, STAT_COUNT, StatValue};
use nfsmw_career::rapsheet::PursuitHistory;
use nfsmw_career::save::ProfileSave;
use nfsmw_career::save::milestones_block::{ArrayError, BinCounter, MilestoneBlock, MilestoneEntry, SpeedTrapEntry};
use nfsmw_career::savefile::AutosaveManager;
use nfsmw_career::settings::{CareerSettings, NewCareer, new_career_with};
use nfsmw_career::stable::Stable;

use super::CareerData;

/// The career being played.
pub struct Career {
    /// The profile name the career was opened with.
    pub alias: String,
    /// The career settings (cash, flags, SMS inbox, gameplay blob). `stage` is kept equal to the Blacklist's
    /// `CurrentBin` by [`Career::profile_bytes`].
    pub settings: CareerSettings,
    /// The car stable.
    pub stable: Stable,
    /// The pursuit history (the statistics block).
    pub history: PursuitHistory,
    /// The Blacklist: the bins, the rival gates and the current bin.
    pub blacklist: Blacklist,
    /// The milestones, with their states and banked values.
    pub milestones: MilestoneSet,
    /// The speed traps, one per [`CareerData::traps`] entry.
    pub traps: Vec<SpeedTrap>,
    /// The markers held: Get Out of Jail Free, Extra Impound Strike, Impound Release. Kept in the popup table of the
    /// settings, which [`Career::profile_bytes`] writes back.
    pub markers: MarkerInventory,
    /// The autosave gate.
    pub autosave: AutosaveManager,
    /// The career tables read from the install.
    pub data: Arc<CareerData>,
    /// The profile file as it was read (or as [`ProfileSave::new_blank`] built it). Its unknown bytes are kept.
    profile: ProfileSave,
}

impl Career {
    /// A new career named `alias`: the settings of `opts` ([`new_career_with`]), an empty stable and history, a
    /// Blacklist at stage 16 and every milestone and speed trap locked.
    pub fn new(alias: &str, data: Arc<CareerData>, opts: &NewCareer) -> Result<Career> {
        let mut settings = new_career_with(opts);
        let blacklist = Blacklist::new(data.bins.clone(), settings.stage).context("building the Blacklist")?;
        let (milestones, traps) = fresh_progress(&data);
        let markers = MarkerInventory::empty();
        let profile = ProfileSave::new_blank(alias).context("building the blank profile")?;
        let block = new_block(&data).context("laying out the new career's milestone block")?;
        block.write_into(&mut settings.gameplay_blob[..]).context("storing the milestone block")?;
        Ok(Career {
            alias: alias.to_string(),
            settings,
            stable: Stable::empty(),
            history: PursuitHistory::default(),
            blacklist,
            milestones,
            traps,
            markers,
            autosave: AutosaveManager::default(),
            data,
            profile,
        })
    }

    /// Loads a career from the bytes of a profile file. The file is checked by [`ProfileSave::parse`] (length, magic,
    /// header checksum, MD5). The milestones and traps are read from the milestone block when it can be laid out,
    /// and built fresh from `data` when it cannot.
    pub fn from_profile_bytes(alias: &str, bytes: &[u8], data: Arc<CareerData>) -> Result<Career> {
        let profile = ProfileSave::parse(bytes).context("reading the profile file")?;
        let settings = profile.settings().context("reading the career settings")?;
        let stable = profile.stable().context("reading the car stable")?;
        let history = profile.history().context("reading the pursuit history")?;
        let mut blacklist = Blacklist::new(data.bins.clone(), settings.stage).context("building the Blacklist")?;
        let total = u32::try_from(stable.total_bounty()).unwrap_or(u32::MAX);
        // The Blacklist re-checks its rival gates against the bounty; the entries it returns are not replayed here.
        let _ = blacklist.bounty_changed(total);
        let (milestones, traps) = restored_progress(&settings, &data).unwrap_or_else(|| fresh_progress(&data));
        let markers = MarkerInventory::from_table(&settings.inventory[..]).context("reading the marker inventory")?;
        Ok(Career {
            alias: alias.to_string(),
            settings,
            stable,
            history,
            blacklist,
            milestones,
            traps,
            markers,
            autosave: AutosaveManager::default(),
            data,
            profile,
        })
    }

    /// Writes the rule state back into the kept profile file and returns the whole file, with the profile MD5 and the
    /// three checksums recomputed.
    ///
    /// A part that cannot be written is logged and its bytes stay as they were: the Blacklist's stage is always
    /// written, the milestone block only when its header counts already hold the records (see [`write_progress`]).
    pub fn profile_bytes(&mut self) -> Vec<u8> {
        self.blacklist.apply_to_settings(&mut self.settings);
        if let Err(e) = self.write_progress() {
            log::warn!("career progress not written to the milestone block: {e:#}");
        }
        self.settings.inventory[..].copy_from_slice(&self.markers.to_table());
        if let Err(e) = self.profile.set_settings(&self.settings) {
            log::warn!("career settings not written to the profile: {e}");
        }
        self.profile.set_stable(&self.stable);
        if let Err(e) = self.profile.set_history(&self.history) {
            log::warn!("pursuit history not written to the profile: {e}");
        }
        self.profile.to_bytes()
    }

    /// The Blacklist's current bin, the stage the career is at (16 for the prologue, 1 for the last rival).
    pub fn stage(&self) -> u8 {
        self.blacklist.current_bin()
    }

    /// The cash on hand.
    pub fn cash(&self) -> u32 {
        self.settings.cash
    }

    /// Copies the milestones, speed traps, statistics and bin counters into the milestone block of the gameplay blob,
    /// then refreshes the block digest.
    ///
    /// The block keeps its records: an entry is updated only when its key matches a milestone or trap in memory, and
    /// the type key of a milestone entry (not held in memory) is kept. A blob whose header counts are zero has no
    /// records to update, so the progress is not stored: laying out new records is not implemented.
    fn write_progress(&mut self) -> Result<()> {
        let mut block = MilestoneBlock::from_bytes(&self.settings.gameplay_blob[..])?;
        let layout = block.layout()?;
        // A blank blob has no bin counter block (its size is 0), so there is no record area to store progress in yet.
        if layout.bin_counters_len < 4 {
            return Ok(());
        }
        let mut entries = block.milestones()?;
        for entry in entries.iter_mut() {
            let Some(m) = self.milestones.milestones().iter().find(|m| m.def.key == entry.key) else {
                continue;
            };
            let type_key = entry.type_key;
            entry.update_from_milestone(m, type_key);
        }
        let mut trap_entries = block.speed_traps()?;
        for entry in trap_entries.iter_mut() {
            let Some(trap) = self.traps.iter().find(|t| t.key() == entry.key) else {
                continue;
            };
            entry.update_from_trap(trap);
        }
        let mut counters = block.bin_counters()?;
        for counter in counters.iter_mut() {
            counter.set_progress(&self.milestones, &self.traps);
        }
        let mut nodes = block.stats()?;
        for (node, value) in nodes.iter_mut().zip(self.milestones.stats().iter()) {
            node.current = value.current;
            node.best = value.best;
        }
        block.set_milestones(&entries)?;
        block.set_speed_traps(&trap_entries)?;
        block.set_bin_counters(&counters)?;
        block.set_stats(&nodes)?;
        block.refresh_digest();
        block.write_into(&mut self.settings.gameplay_blob[..])?;
        Ok(())
    }
}

/// The milestone block of a new career: every milestone and speed trap locked, nothing banked, one bin counter per
/// Blacklist bin. The statistic node keys and the milestone type keys are not in [`CareerData`], so they are `0`
/// (unconfirmed; see [`MilestoneBlock::new_career`]).
fn new_block(data: &CareerData) -> Result<MilestoneBlock, ArrayError> {
    let milestones: Vec<MilestoneEntry> =
        data.milestones.iter().map(|&def| MilestoneEntry::new_locked(def, 0)).collect();
    let traps: Vec<SpeedTrapEntry> = data.traps.iter().map(|&def| SpeedTrapEntry::new_locked(def)).collect();
    let bins: Vec<BinCounter> = data.bins.iter().map(|bin| BinCounter::new_career(u16::from(bin.number))).collect();
    MilestoneBlock::new_career(&milestones, &traps, &bins)
}

/// The milestones and speed traps of a new career: all locked, none banked (`career-milestones.md` section 7).
fn fresh_progress(data: &CareerData) -> (MilestoneSet, Vec<SpeedTrap>) {
    let milestones = MilestoneSet::new(&data.milestones);
    let traps = data.traps.iter().map(|&def| SpeedTrap::new(def)).collect();
    (milestones, traps)
}

/// The milestones and speed traps read from the milestone block of `settings`. `None` when the block cannot be laid
/// out, which is the case for a blank profile; the caller then starts fresh.
///
/// A milestone or trap with no entry in the block (or whose entry does not match its definition) starts fresh. The
/// statistics are taken in the block's node order, which the statistic table of `StatType::ALL` is assumed to follow.
/// The pursuit summary is not in the parts of the block this crate decodes, so it starts empty.
fn restored_progress(settings: &CareerSettings, data: &CareerData) -> Option<(MilestoneSet, Vec<SpeedTrap>)> {
    let block = MilestoneBlock::from_bytes(&settings.gameplay_blob[..]).ok()?;
    block.layout().ok()?;
    let entries = block.milestones().ok()?;
    let trap_entries = block.speed_traps().ok()?;
    let nodes = block.stats().ok()?;
    let milestones: Vec<Milestone> = data
        .milestones
        .iter()
        .map(|&def| {
            entries
                .iter()
                .find(|e| e.key == def.key)
                .and_then(|e| e.to_milestone(def))
                .unwrap_or_else(|| Milestone::new(def))
        })
        .collect();
    let traps = data
        .traps
        .iter()
        .map(|&def| match trap_entries.iter().find(|e| e.key == def.key) {
            Some(e) => SpeedTrap::restore(def, e.flags, e.recorded_mps),
            None => SpeedTrap::new(def),
        })
        .collect();
    let mut stats = [StatValue::default(); STAT_COUNT];
    for (slot, node) in stats.iter_mut().zip(nodes.iter()) {
        *slot = node.value();
    }
    Some((MilestoneSet::from_parts(milestones, stats, Vec::new()), traps))
}

#[cfg(test)]
#[path = "state_tests.rs"]
mod tests;
