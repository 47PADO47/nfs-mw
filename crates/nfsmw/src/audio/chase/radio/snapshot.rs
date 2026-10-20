//! The world's chase feed as the machine's snapshot, and the changes between two feeds as notices.

use blackbox_pursuit::speech::{
    Car, CopInfo, HeliInfo, Impact, Level, Notice, PlayerInfo, PursuitInfo, RoadblockInfo, Snapshot, Strategy,
};
use blackbox_pursuit::{Infraction, InfractionLog};
use glam::Vec3;
use nfsmw_data::speech_chase::LevelSpeech;

use super::tables::{road_battalion, vehicle_battalion};
use crate::audio::PursuitFeed;
use crate::audio::chase::feed::{CopFeed, Formation, HeliFeed, RoadblockFeed};
use crate::audio::speech::params::values::car_type;

/// Rain at or above this reads as bad weather (the helicopter's bail-out wording). **[guess]**, as the cue's threshold.
const WET: f32 = 0.5;

fn strategy(formation: Formation) -> Strategy {
    match formation {
        Formation::Follow => Strategy::Follow,
        Formation::StaggerFollow => Strategy::StaggerFollow,
        Formation::Pit => Strategy::Pit,
        Formation::BoxIn => Strategy::BoxIn,
        Formation::RollingBlock => Strategy::RollingBlock,
        Formation::Herd => Strategy::Herd,
    }
}

fn cop(feed: &CopFeed) -> CopInfo {
    CopInfo {
        id: feed.id,
        active: feed.active,
        in_view: feed.in_view,
        los: feed.los,
        ahead: feed.ahead,
        distance: feed.distance,
        speed_mph: feed.speed_mph,
        position: Vec3::from_array(feed.position),
        health: feed.health,
        destroyed: feed.destroyed,
        in_formation: feed.in_formation,
        in_position: feed.in_position,
        offset: (feed.offset[0], feed.offset[1]),
        vehicle_battalion: vehicle_battalion(feed.vehicle),
        road_battalion: road_battalion(feed.road),
        cross: feed.vehicle == crate::audio::chase::feed::Vehicle::Cross,
        roadblock: feed.roadblock,
    }
}

fn heli(feed: &HeliFeed) -> HeliInfo {
    HeliInfo {
        id: feed.id,
        active: feed.active,
        los: feed.los,
        distance: feed.distance,
        health: 1.0,
        fuel_secs: Some(feed.fuel_secs),
    }
}

fn roadblock(feed: &RoadblockFeed) -> RoadblockInfo {
    RoadblockInfo {
        id: feed.id,
        visual: feed.visual,
        spike_strips: feed.spike_strips,
        cops_damaged: 0,
        cops_destroyed: 0,
        distance: feed.distance,
    }
}

/// The machine's snapshot of a running chase. The player's car is described only when the feed knows its type
/// (`commentable`: a car type of 0 or the one nobody recorded is not described).
pub fn snapshot(feed: &PursuitFeed, level: &LevelSpeech) -> Snapshot {
    let speech = &feed.speech;
    let player = &speech.player;
    let described = player.car_type != 0 && player.car_type != car_type::PAGANI;
    Snapshot {
        pursuit: PursuitInfo {
            exists: true,
            other_target: false,
            re_acquiring: false,
            perp_in_sight: feed.intensity >= 1.0,
            cooling_down: speech.status.cooling_down,
            cooldown_remaining: feed.secs_to_evade,
            evaded: false,
            strategy: strategy(speech.status.formation),
            collapse_active: speech.status.collapse_active,
            finisher_active: speech.status.finisher_active,
            evade_level: speech.status.evade_level,
            backup_eta: 0.0,
            busted: speech.status.busted,
            cops_destroyed: feed.cops_destroyed,
            pursuits: 1,
            heat: feed.heat as f32,
        },
        player: PlayerInfo {
            speed_mph: feed.player_mph,
            position: Vec3::from_array(player.position),
            velocity_dir: Vec3::from_array(player.velocity_dir),
            forward: Vec3::from_array(player.forward),
            road: player.road,
            road_forward: None,
            brake: 0.0,
            handbrake: 0.0,
            up_y: 1.0,
            car: described.then_some(Car { kind: player.car_type, colour: player.car_colour }),
            commentable: described,
            metric: false,
            offroad_id: None,
        },
        cops: speech.cops().iter().map(cop).collect(),
        heli: speech.heli.as_ref().map(heli),
        roadblock: speech.roadblock.as_ref().map(roadblock),
        level: Level {
            roadblocks: level.roadblocks,
            heli_roadblocks: level.heli_roadblocks,
            spikes: level.spikes,
            cts_for_911: level.cts_for_911,
            hits_for_911: level.hits_for_911,
            lifetime_911: level.lifetime_911,
            cops_enabled: true,
            roaming: false,
            ddays: false,
            pursuit_race: false,
            lockout_remaining: 0.0,
        },
        racers_near: false,
        weather: feed.rain_intensity >= WET,
    }
}

/// What the machine remembers of the last feed to find what changed.
#[derive(Debug, Clone, Copy, Default)]
pub struct Prev {
    infractions: InfractionLog,
    player_hit: bool,
    seen: bool,
}

/// The notices the change from the last feed to this one makes: a new infraction (an assault is also a hit on the
/// nearest cop, the world not telling which one or how), and the player hitting traffic.
pub fn notices(prev: &mut Prev, feed: &PursuitFeed) -> Vec<Notice> {
    let mut out = Vec::new();
    let now = feed.speech.status.infractions;
    if !prev.seen {
        // The first feed of a chase: what the log already holds is the chase's reason (the latest stands for the
        // message the original receives); the hit memory only records.
        if let Some(latest) = now.iter().last() {
            out.push(Notice::Infraction(latest));
        }
    } else {
        let before = prev.infractions;
        let added = Infraction::ALL.into_iter().filter(|&i| now.contains(i) && !before.contains(i));
        for infraction in added {
            out.push(Notice::Infraction(infraction));
            if infraction == Infraction::Assault {
                let nearest =
                    feed.speech.cops().iter().filter(|c| c.active).min_by(|a, b| a.distance.total_cmp(&b.distance));
                if let Some(cop) = nearest {
                    out.push(Notice::PlayerHitCop {
                        cop: cop.id,
                        impact: Impact::SideSwipe,
                        intensity: 0.4,
                        deliberate: true,
                    });
                }
            }
        }
        if feed.player_hit && !prev.player_hit {
            out.push(Notice::PlayerHitTraffic { intensity: 0.6, semi: false });
        }
    }
    *prev = Prev { infractions: now, player_hit: feed.player_hit, seen: true };
    out
}

impl Prev {
    /// The chase is over: the next one starts fresh.
    pub fn forget(&mut self) {
        *self = Self::default();
    }
}
