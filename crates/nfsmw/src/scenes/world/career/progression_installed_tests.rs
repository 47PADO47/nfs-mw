//! An installed Blacklist slice through the world's own race-result path: bin 15's races (a loss, then wins), its
//! rival gate, a lost then won rival challenge, the gauntlet rewards, and save/reload at each step.
use std::sync::Arc;

use game_install::GameDir;
use nfsmw_career::blacklist::{BinState, enter_first_bin};
use nfsmw_career::settings::NewCareer;
use nfsmw_data::gameplay::RewardRecord;

use super::events::{RaceFinish, apply_race_finish, arrival_pending};
use super::phone::PhoneState;
use crate::career::{Career, CareerData, race_event_id};

struct Install {
    data: Arc<CareerData>,
    gameplay: Arc<nfsmw_data::gameplay::GameplayData>,
}

impl Install {
    fn race(&self, event: &str) -> &nfsmw_data::gameplay::Race {
        self.gameplay.races.iter().find(|race| race_event_id(race) == event).unwrap_or_else(|| panic!("race {event}"))
    }

    fn finish(&self, career: &mut Career, phone: &mut PhoneState, event: &str, place: u8) -> Vec<u8> {
        let race = self.race(event);
        let rewards: (Vec<RewardRecord>, bool) = (race.story.rewards.clone(), race.story.defer_rewards);
        let finished = RaceFinish {
            event,
            place,
            cash_value: race.params.cash_value,
            series_complete: false,
            rewards: Some(&rewards),
        };
        apply_race_finish(career, phone, &finished).bins_done
    }

    /// Saves the career and loads it back, as a quit to the menu and a later resume do.
    fn reload(&self, career: &mut Career) -> Career {
        let bytes = career.profile_bytes();
        Career::from_profile_bytes(&career.alias, &bytes, self.data.clone()).expect("the saved profile loads")
    }
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR): installed Blacklist, races and rewards"]
fn installed_bin_15_runs_from_first_race_to_beaten_rival_across_reloads() {
    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").expect("set NFSMW_GAME_DIR")).unwrap();
    let install =
        Install { data: CareerData::cached(&dir).unwrap(), gameplay: nfsmw_data::gameplay::cached(&dir).unwrap() };
    let mut career = Career::new("e2e", install.data.clone(), &NewCareer::default()).unwrap();
    let mut phone = PhoneState::default();
    {
        let c = &mut career;
        enter_first_bin(&mut c.settings, &mut c.blacklist, &mut c.milestones, &mut c.traps);
    }
    assert_eq!(career.stage(), 15);
    let def = career.blacklist.bin(15).unwrap().def.clone();
    eprintln!(
        "bin 15: {} races (need {}), {} challenges, bounty {}, rival races {:?}",
        def.races.len(),
        def.races_needed,
        def.challenges_needed,
        def.bounty_needed,
        def.rival_events
    );
    assert!(def.races.len() >= def.races_needed as usize && !def.rival_events.is_empty());
    let first = def.races[0].clone();

    // A loss counts no win and pays nothing; a restart or quit applies no result at all.
    let cash = career.cash();
    assert!(install.finish(&mut career, &mut phone, &first, 4).is_empty());
    assert_eq!(career.cash(), cash, "a loss pays no race cash");
    assert_eq!(career.blacklist.bin(15).unwrap().state, Some(BinState::RivalLocked));
    let mut career = install.reload(&mut career);
    assert_eq!(career.stage(), 15, "the stage survives a reload");

    // Winning the needed races pays their cash and survives a reload.
    for event in def.races.iter().take(def.races_needed as usize) {
        let before = career.cash();
        install.finish(&mut career, &mut phone, event, 1);
        let paid = install.race(event).params.cash_value as u32;
        assert!(career.cash() >= before, "{event}: a win never takes cash");
        eprintln!("{event}: won, cash {before} -> {} (race value {paid})", career.cash());
    }
    let mut career = install.reload(&mut career);
    let record = career.blacklist.bin(15).unwrap();
    assert!(record.races_won.len() >= def.races_needed as usize, "won races survive a reload: {:?}", record.races_won);

    // The rival's gate also needs its challenges and bounty; the pursuit side reports those.
    for index in 0..def.challenges_needed {
        career.blacklist.challenge_passed(15, &format!("e2e-challenge-{index}"));
    }
    career.blacklist.bounty_changed(def.bounty_needed);
    assert_eq!(career.blacklist.bin(15).unwrap().state, Some(BinState::RivalUnlocked), "the rival opens");

    // A lost rival race leaves the rival standing.
    let rival = def.rival_events.clone();
    assert!(install.finish(&mut career, &mut phone, &rival[0], 2).is_empty());
    assert_eq!(career.stage(), 15);
    assert_eq!(career.rival_beaten, None);

    // Winning every rival race beats the rival, grants the gauntlet and moves the walk to bin 14.
    let before = format!("{:?}|{:?}|{}", career.markers, career.stable, career.cash());
    let mut done = Vec::new();
    for event in &rival {
        done.extend(install.finish(&mut career, &mut phone, event, 1));
    }
    assert_eq!(done, [15], "the last rival race finishes the bin");
    assert_eq!(career.stage(), 14);
    assert_eq!(career.rival_beaten, Some(15), "the safe house runs the arrival sequence next");
    let after = format!("{:?}|{:?}|{}", career.markers, career.stable, career.cash());
    let last = install.race(rival.last().unwrap());
    eprintln!("rival rewards {:?} (deferred {})", last.story.rewards, last.story.defer_rewards);
    if !last.story.rewards.is_empty() && !last.story.defer_rewards {
        assert_ne!(before, after, "the gauntlet's rewards reach the career");
    }
    if last.story.defer_rewards {
        assert_eq!(career.popups.entries().len(), last.story.rewards.len().min(6), "deferred to the markers screen");
    }
    assert!(arrival_pending(&career), "the roaming autosave waits for the arrival sequence and its markers");

    // Everything survives a final save and reload.
    let back = install.reload(&mut career);
    assert_eq!(back.stage(), 14);
    let record = back.blacklist.bin(15).unwrap();
    assert_eq!(record.state, Some(BinState::Done));
    assert_eq!(record.rival_races_won.len(), rival.len());
    assert_eq!(back.cash(), career.cash());
    assert!(back.blacklist.bin(14).unwrap().state.is_some(), "bin 14 is entered");
}
