//! The free-roam frame of the chase radio: what the world tells the speech machine while no cop chases, which is what
//! the free-roam 911 call is made of (`docs/specs/ai-pursuit-heat.md` section 5.3, `docs/specs/ai-pursuit-speech.md`
//! section 8). The world fills it in `scenes/world/ai/police/roam_911.rs` and the scene adds the car's look.

use blackbox_pursuit::speech::{Level, Notice, PlayerInfo, Snapshot};
use glam::Vec3;
use nfsmw_data::speech_chase::LevelSpeech;

use super::feed::PlayerFeed;
use crate::audio::speech::params::values::car_type;

/// What the player did that the 911 logic counts.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RoamEvent {
    /// The player hit an object that adds `cost` to the damage bill.
    Smashed { cost: u32 },
    /// The player hit a traffic car with this intensity, 0 to 1.
    HitTraffic { intensity: f32 },
}

/// The free-roam state of the player and the 911 counters' inputs.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RoamFeed {
    pub player: PlayerFeed,
    pub speed_mph: f32,
    /// The session heat, which picks the heat row.
    pub heat: u32,
    /// Seconds left of the cop lockout.
    pub lockout_remaining: f32,
    /// What the player hit since the last frame.
    pub events: Vec<RoamEvent>,
    /// The cops were locked out again (an evade, a paid ticket): the speech restarts.
    pub restarted: bool,
    /// The counters hold something: the machine runs even though it is idle.
    pub armed: bool,
}

/// The machine's snapshot of a free-roam frame: no pursuit, the player, and the heat row's 911 figures.
pub fn snapshot(feed: &RoamFeed, level: &LevelSpeech) -> Snapshot {
    let player = &feed.player;
    let described = player.car_type != 0 && player.car_type != car_type::PAGANI;
    let mut snap = Snapshot::default();
    snap.pursuit.heat = feed.heat as f32;
    snap.player = PlayerInfo {
        speed_mph: feed.speed_mph,
        position: Vec3::from_array(player.position),
        velocity_dir: Vec3::from_array(player.velocity_dir),
        forward: Vec3::from_array(player.forward),
        road: player.road,
        car: described.then_some(blackbox_pursuit::speech::Car { kind: player.car_type, colour: player.car_colour }),
        commentable: described,
        up_y: 1.0,
        ..PlayerInfo::default()
    };
    snap.level = Level {
        roadblocks: level.roadblocks,
        heli_roadblocks: level.heli_roadblocks,
        spikes: level.spikes,
        cts_for_911: level.cts_for_911,
        hits_for_911: level.hits_for_911,
        lifetime_911: level.lifetime_911,
        cops_enabled: true,
        roaming: true,
        lockout_remaining: feed.lockout_remaining,
        ..Level::default()
    };
    snap
}

/// The notices of a free-roam frame, in the order the world saw them.
pub fn notices(feed: &RoamFeed) -> Vec<Notice> {
    let mut out = Vec::new();
    if feed.restarted {
        out.push(Notice::Restart);
    }
    for event in &feed.events {
        out.push(match *event {
            RoamEvent::Smashed { cost } => Notice::Smashed { cost: cost.min(i32::MAX as u32) as i32 },
            RoamEvent::HitTraffic { intensity } => Notice::PlayerHitTraffic { intensity, semi: false },
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_snapshot_is_a_roaming_one_with_the_rows_figures() {
        let level = LevelSpeech { cts_for_911: 400, hits_for_911: 3, lifetime_911: 45.0, ..Default::default() };
        let feed = RoamFeed { heat: 3, lockout_remaining: 12.0, ..Default::default() };
        let snap = snapshot(&feed, &level);
        assert!(!snap.pursuit.exists);
        assert!(snap.level.roaming && snap.level.cops_enabled);
        assert_eq!((snap.level.cts_for_911, snap.level.hits_for_911), (400, 3));
        assert_eq!(snap.level.lifetime_911, 45.0);
        assert_eq!(snap.level.lockout_remaining, 12.0);
        assert_eq!(snap.pursuit.heat, 3.0);
    }

    #[test]
    fn the_events_become_the_machines_notices_in_order() {
        let feed = RoamFeed {
            events: vec![RoamEvent::Smashed { cost: 150 }, RoamEvent::HitTraffic { intensity: 0.7 }],
            restarted: true,
            ..Default::default()
        };
        let notices = notices(&feed);
        assert_eq!(notices.len(), 3);
        assert!(matches!(notices[0], Notice::Restart));
        assert!(matches!(notices[1], Notice::Smashed { cost: 150 }));
        assert!(matches!(notices[2], Notice::PlayerHitTraffic { intensity, semi: false } if intensity == 0.7));
    }

    fn level() -> LevelSpeech {
        LevelSpeech { cts_for_911: 400, hits_for_911: 3, lifetime_911: 45.0, ..Default::default() }
    }

    fn frame(events: Vec<RoamEvent>) -> RoamFeed {
        RoamFeed { heat: 1, armed: !events.is_empty(), events, ..Default::default() }
    }

    /// Runs the radio for a few tenths of a second and collects what it asked for.
    fn run(radio: &mut super::super::ChaseRadio, mut feeds: Vec<RoamFeed>) -> Vec<blackbox_pursuit::speech::Line> {
        let mut lines = Vec::new();
        for _ in 0..8 {
            let feed = if feeds.is_empty() { frame(Vec::new()) } else { feeds.remove(0) };
            radio.set_roam(Some(feed));
            let out = radio.run(0.1, None, blackbox_pursuit::EndReason::None, &level(), None);
            lines.extend(out.requests.iter().map(|request| request.line));
        }
        lines
    }

    #[test]
    fn smashed_property_in_free_roam_makes_the_dispatcher_report_911() {
        let mut radio = super::super::ChaseRadio::new(7);
        let lines = run(&mut radio, vec![frame(vec![RoamEvent::Smashed { cost: 500 }])]);
        assert!(lines.contains(&blackbox_pursuit::speech::Line::Disp911Report), "{lines:?}");
        assert_eq!(radio.take_911_call(), None, "the call starts when the line has been said, not when it is asked");
    }

    #[test]
    fn a_little_damage_in_free_roam_says_nothing() {
        let mut radio = super::super::ChaseRadio::new(7);
        let lines = run(&mut radio, vec![frame(vec![RoamEvent::Smashed { cost: 150 }])]);
        assert!(lines.is_empty(), "{lines:?}");
    }

    #[test]
    fn hard_traffic_hits_make_the_report_and_light_ones_do_not() {
        let mut radio = super::super::ChaseRadio::new(7);
        let light = RoamEvent::HitTraffic { intensity: 0.2 };
        let hard = RoamEvent::HitTraffic { intensity: 0.9 };
        let lines = run(&mut radio, vec![frame(vec![light, light, light, light])]);
        assert!(!lines.contains(&blackbox_pursuit::speech::Line::Disp911Report), "{lines:?}");
        let lines = run(&mut radio, vec![frame(vec![hard, hard, hard])]);
        assert!(lines.contains(&blackbox_pursuit::speech::Line::Disp911Report), "{lines:?}");
    }

    #[test]
    fn an_idle_free_roam_frame_does_no_work() {
        let mut radio = super::super::ChaseRadio::new(7);
        radio.set_roam(Some(frame(Vec::new())));
        let out = radio.run(0.1, None, blackbox_pursuit::EndReason::None, &level(), None);
        assert!(out.is_empty());
    }
}
