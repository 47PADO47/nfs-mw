//! Tests for what the notifications show and play: the view built from the queue, the cue and rumble raised once per
//! notice, the feedback lane's chips, and the gates on the HUD's sound and the pad's rumble.

use bevy_ecs::entity::Entity;

use super::feedback::{COUNT_UP, MIN_UP, NEAR_MISS_HOLD, NITROUS_PULSE};
use super::output::{TAP, THUMP, plays_cues, rumble, rumble_pad, trigger};
use super::*;
use crate::settings::hud::HudSettings;
use crate::settings::{SpeedUnits, UiStyle};

const DT: f32 = 1.0 / 60.0;

/// A HUD state of frame `frame` carrying `events`.
fn frame(frame: u64, events: Vec<HudEvent>) -> HudState {
    HudState { events, events_frame: frame, ..Default::default() }
}

/// Takes `events` as frame `n` and ticks one frame.
fn step(notifications: &mut HudNotifications, n: u64, events: Vec<HudEvent>) {
    notifications.take_events(&frame(n, events), NotificationLevel::All);
    notifications.tick(DT, false);
}

/// Ticks `seconds` of frames with no event, from frame `n`.
fn idle(notifications: &mut HudNotifications, n: &mut u64, seconds: f32) {
    let frames = (seconds / DT).ceil() as u64;
    for _ in 0..frames {
        *n += 1;
        step(notifications, *n, Vec::new());
    }
}

#[test]
fn the_view_resolves_the_visible_notices_most_urgent_first() {
    let mut notifications = HudNotifications::default();
    step(&mut notifications, 1, vec![HudEvent::ContactLost, HudEvent::HeatUp { level: 4 }]);
    let view = notifications.view(SpeedUnits::Metric, true);
    let texts: Vec<&str> = view.notices.iter().map(|notice| notice.text.as_str()).collect();
    assert_eq!(texts, ["Heat level 4", "Visual contact lost"], "High above Medium");
    let lost = &view.notices[1];
    assert_eq!(lost.secondary.as_deref(), Some("Stay out of sight"));
    assert_eq!((lost.id, lost.priority, lost.category), (NoticeId::ContactLost, Priority::Medium, Category::Pursuit));
    assert_eq!(lost.icon, "eye-off");
    assert!((lost.age - DT).abs() < 1e-4, "one frame old");
    assert!((lost.remaining - (4.0 - DT)).abs() < 1e-4, "four seconds less one frame");
    assert_eq!(lost.count, 1);
    assert!(view.chip.is_none() && view.nitrous_age.is_none());
}

#[test]
fn a_repeat_refreshes_the_notice_without_restarting_it() {
    let mut notifications = HudNotifications::default();
    let mut n = 1;
    step(&mut notifications, n, vec![HudEvent::HeatUp { level: 2 }]);
    idle(&mut notifications, &mut n, 1.0);
    n += 1;
    step(&mut notifications, n, vec![HudEvent::HeatUp { level: 3 }]);
    let view = notifications.view(SpeedUnits::Metric, true);
    assert_eq!(view.notices.len(), 1, "one notice, refreshed");
    let heat = &view.notices[0];
    assert_eq!(heat.text, "Heat level 3", "the newer value wins");
    assert_eq!(heat.count, 2);
    assert!(heat.age > 1.0, "the age runs on: the entry is not replayed");
    assert!(heat.remaining > 2.9, "the time is pushed out");
}

#[test]
fn a_notice_plays_its_cue_and_rumble_once_and_not_on_a_refresh() {
    let mut notifications = HudNotifications::default();
    step(&mut notifications, 1, vec![HudEvent::HeatUp { level: 2 }]);
    assert_eq!(notifications.take_cues(), [AudioCue::Escalation]);
    assert_eq!(notifications.take_haptics(), [Haptic::Light]);
    let mut n = 1;
    idle(&mut notifications, &mut n, 1.0);
    n += 1;
    step(&mut notifications, n, vec![HudEvent::HeatUp { level: 3 }]);
    assert!(notifications.take_cues().is_empty(), "a refresh plays nothing");
    assert!(notifications.take_haptics().is_empty());
    idle(&mut notifications, &mut n, 2.0);
    assert!(notifications.take_cues().is_empty(), "a held notice plays nothing more");
}

#[test]
fn a_waiting_notice_plays_when_it_shows_and_a_cue_does_not_stutter() {
    let mut notifications = HudNotifications::default();
    let damage = vec![HudEvent::TirePunctured, HudEvent::HeavyDamage, HudEvent::VehicleCritical];
    step(&mut notifications, 1, damage);
    assert_eq!(notifications.take_cues(), [AudioCue::CriticalDamage], "three at once play the cue once");
    assert_eq!(notifications.take_haptics(), [Haptic::Strong, Haptic::Strong, Haptic::Strong]);
    step(&mut notifications, 2, vec![HudEvent::Checkpoint { number: 1, total: 4 }]);
    assert!(notifications.take_cues().is_empty(), "the checkpoint waits for room, and so does its cue");
    let mut n = 2;
    idle(&mut notifications, &mut n, 4.0);
    assert_eq!(notifications.take_cues(), [AudioCue::Checkpoint], "it plays when the notice comes up");
}

#[test]
fn the_nitrous_plays_at_once_and_pulses_the_cluster() {
    let mut notifications = HudNotifications::default();
    step(&mut notifications, 1, vec![HudEvent::NitrousOn]);
    assert_eq!(notifications.take_cues(), [AudioCue::Nitrous]);
    assert!(notifications.view(SpeedUnits::Metric, true).nitrous_age.is_some_and(|age| age < 0.1));
    let mut n = 1;
    idle(&mut notifications, &mut n, NITROUS_PULSE);
    assert!(notifications.view(SpeedUnits::Metric, true).nitrous_age.is_none(), "the pulse is over");
}

#[test]
fn near_misses_count_up_on_one_chip_and_are_no_notice() {
    let mut notifications = HudNotifications::default();
    let mut n = 1;
    step(&mut notifications, n, vec![HudEvent::NearMiss]);
    idle(&mut notifications, &mut n, 1.5);
    n += 1;
    step(&mut notifications, n, vec![HudEvent::NearMiss]);
    let view = notifications.view(SpeedUnits::Metric, true);
    assert!(view.notices.is_empty(), "a near miss is not a queue notice");
    let chip = view.chip.expect("the near-miss chip");
    assert_eq!((chip.kind, chip.text.as_str(), chip.count), (ChipKind::NearMiss, "Near miss", 2));
    assert!(chip.age > 1.5, "the repeat did not replay the entry");
    idle(&mut notifications, &mut n, NEAR_MISS_HOLD);
    assert!(notifications.view(SpeedUnits::Metric, true).chip.is_none(), "gone once the near misses stop");
}

#[test]
fn bounty_gains_add_up_and_count_up() {
    let mut notifications = HudNotifications::default();
    let mut n = 1;
    step(&mut notifications, n, vec![HudEvent::BountyGained { amount: 1_000 }]);
    idle(&mut notifications, &mut n, 0.3);
    n += 1;
    step(&mut notifications, n, vec![HudEvent::BountyGained { amount: 1_500 }]);
    let counting = notifications.view(SpeedUnits::Metric, true).chip.expect("the bounty chip");
    assert_ne!(counting.text, "+2,500", "still counting up");
    assert_eq!(notifications.view(SpeedUnits::Metric, false).chip.expect("the chip").text, "+2,500", "no count-up");
    idle(&mut notifications, &mut n, COUNT_UP);
    let chip = notifications.view(SpeedUnits::Metric, true).chip.expect("the bounty chip");
    assert_eq!((chip.kind, chip.text.as_str(), chip.count), (ChipKind::Bounty, "+2,500", 2));
}

#[test]
fn a_chip_of_another_kind_waits_until_the_one_up_has_had_its_time() {
    let mut notifications = HudNotifications::default();
    let mut n = 1;
    step(&mut notifications, n, vec![HudEvent::NearMiss]);
    n += 1;
    step(&mut notifications, n, vec![HudEvent::SpeedTrap { kmh: 212.0, passed: true }]);
    let kind = |notifications: &HudNotifications| notifications.view(SpeedUnits::Metric, true).chip.map(|c| c.kind);
    assert_eq!(kind(&notifications), Some(ChipKind::NearMiss));
    idle(&mut notifications, &mut n, MIN_UP);
    assert_eq!(kind(&notifications), Some(ChipKind::SpeedTrap));
    let chip = notifications.view(SpeedUnits::Imperial, true).chip.expect("the speed-trap chip");
    assert_eq!(chip.text, "Speed trap 132 MPH", "in the player's units");
}

#[test]
fn the_lane_is_quiet_with_notifications_off() {
    let mut notifications = HudNotifications::default();
    notifications.take_events(&frame(1, vec![HudEvent::NearMiss]), NotificationLevel::Off);
    notifications.tick(DT, false);
    assert!(notifications.view(SpeedUnits::Metric, true).chip.is_none());
}

#[test]
fn the_cues_are_the_custom_hud_s_while_it_shows() {
    let shown = HudState { visible: true, ..HudState::default() };
    assert!(plays_cues(UiStyle::Custom, true, &shown));
    assert!(!plays_cues(UiStyle::Original, true, &shown), "not the original HUD's");
    assert!(!plays_cues(UiStyle::Remastered, true, &shown));
    assert!(!plays_cues(UiStyle::Custom, false, &shown), "not with the HUD off");
    assert!(!plays_cues(UiStyle::Custom, true, &HudState { visible: false, ..shown }), "not while it is hidden");
}

#[test]
fn every_cue_is_a_front_end_trigger_the_install_plays() {
    let audible = [0, 2, 4, 5, 7, 10, 11, 89];
    let cues = [
        AudioCue::Escalation,
        AudioCue::Hazard,
        AudioCue::ContactLost,
        AudioCue::Cooldown,
        AudioCue::Escaped,
        AudioCue::Busted,
        AudioCue::Checkpoint,
        AudioCue::MissedCheckpoint,
        AudioCue::CriticalDamage,
        AudioCue::Nitrous,
    ];
    for cue in cues {
        assert!(audible.contains(&trigger(cue)), "{cue:?} plays a silent trigger");
    }
}

#[test]
fn the_pad_rumbles_only_with_haptics_on() {
    let pad = Some(Entity::PLACEHOLDER);
    assert!(!HudSettings::default().haptics, "haptics are off by default");
    assert_eq!(rumble_pad(&HudSettings::default(), pad), None);
    let on = HudSettings { haptics: true, ..HudSettings::default() };
    assert_eq!(rumble_pad(&on, pad), pad);
    assert_eq!(rumble_pad(&on, None), None, "no pad, no rumble");
    assert_eq!(rumble(Haptic::Light), TAP);
    assert_eq!(rumble(Haptic::Strong), THUMP);
    assert!(TAP.0.weak_motor > 0.0 && TAP.0.strong_motor == 0.0, "a light tap is the weak motor");
    assert!(THUMP.0.strong_motor > TAP.0.weak_motor && THUMP.1 > TAP.1, "a thump is stronger and longer");
}
