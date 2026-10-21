//! The observer, the arrest, the roadblock chatter and the free-roam 911 call.

use glam::Vec3;

use super::super::*;
use super::support::*;

fn running(seed: u64) -> Chase {
    let mut chase = Chase::new(seed);
    chase.snap.cops = vec![cop(10, 40.0), cop(11, 70.0)];
    chase.run(16.0);
    assert_eq!(chase.machine.focus(), Focus::Strategy);
    chase
}

#[test]
fn a_full_busted_bar_brings_the_bullhorn_and_mutes_the_radio() {
    let mut chase = running(1);
    chase.snap.pursuit.busted = 0.9;
    chase.run(2.0);
    let horn = chase.radio.first(Line::BullhornArrest).expect("bullhorn arrest");
    assert!(horn.speaker != 0 && horn.speaker != DISPATCH);
    assert!(chase.radio.effects.contains(&Effect::BustedLevel(0.9)));
}

#[test]
fn the_helicopter_takes_the_bullhorn_when_it_is_close() {
    let mut chase = running(2);
    chase.snap.heli = Some(HeliInfo { id: 99, active: true, los: true, distance: 60.0, health: 1.0, fuel_secs: None });
    chase.run(2.0);
    chase.snap.pursuit.busted = 0.95;
    chase.run(2.0);
    let horn = chase.radio.first(Line::HeliBullhornArrest).expect("the helicopter's bullhorn");
    assert_eq!(horn.speaker, HELI);
}

#[test]
fn hitting_the_world_hard_is_remarked_on_with_a_low_heat_wording_first() {
    let mut chase = running(3);
    chase.notices.push(Notice::PlayerHitWorld { intensity: 0.6, crash: true });
    chase.run(2.0);
    let first = chase.radio.first(Line::CollisionWorld).expect("collision");
    assert_eq!(first.params.world_object, Some(WorldObject::LowHeat));
    assert_eq!(first.params.units, Some(Units::Multiple));
}

#[test]
fn a_slight_scrape_is_not_a_crash() {
    let mut chase = running(4);
    chase.notices.push(Notice::PlayerHitWorld { intensity: 0.6, crash: false });
    chase.run(2.0);
    assert_eq!(chase.radio.count(Line::CollisionWorld), 0);
}

#[test]
fn hitting_traffic_hard_is_remarked_on_and_counted() {
    let mut chase = running(5);
    chase.notices.push(Notice::PlayerHitTraffic { intensity: 0.9, semi: false });
    chase.run(2.0);
    let line = chase.radio.first(Line::CollWorldCivi).expect("civilian collision");
    assert_eq!(line.params.intensity, Some(Intensity::High));
}

#[test]
fn hitting_a_semi_names_it() {
    let mut chase = running(6);
    chase.notices.push(Notice::PlayerHitTraffic { intensity: 0.8, semi: true });
    chase.run(2.0);
    let line = chase.radio.first(Line::CollisionWorld).expect("collision with a semi");
    assert_eq!(line.params.world_object, Some(WorldObject::Semi));
}

#[test]
fn ramming_a_cop_gets_a_reaction_in_proportion() {
    let mut chase = running(7);
    chase.notices.push(Notice::PlayerHitCop { cop: 10, impact: Impact::TBone, intensity: 0.3, deliberate: true });
    chase.run(1.0);
    let line = chase.radio.first(Line::RamTBone).expect("t-bone reaction");
    assert_eq!(line.params.intensity, Some(Intensity::Normal));
    let mut chase = running(8);
    chase.notices.push(Notice::PlayerHitCop { cop: 10, impact: Impact::HeadOn, intensity: 0.9, deliberate: true });
    chase.run(1.0);
    assert!(chase.radio.first(Line::InterruptViolent).is_some());
}

#[test]
fn a_hit_on_a_braking_cop_is_no_ram() {
    let mut chase = running(9);
    chase.notices.push(Notice::PlayerHitCop { cop: 10, impact: Impact::RearEnd, intensity: 0.9, deliberate: false });
    chase.run(1.0);
    assert_eq!(chase.radio.count(Line::InterruptViolent) + chase.radio.count(Line::InterruptExpletive), 0);
}

#[test]
fn a_cop_hitting_traffic_twice_bails_out_with_heavy_traffic() {
    let mut chase = running(10);
    chase.run(50.0);
    chase.notices.push(Notice::CopHitTraffic { cop: 10 });
    chase.run(1.0);
    let bail = chase.radio.first(Line::Bailout).expect("bailout");
    assert_eq!(bail.params.bailout, Some(Bailout::HeavyTraffic));
}

#[test]
fn a_big_jump_and_a_roll_are_remarked_on() {
    let mut chase = running(11);
    chase.snap.player.speed_mph = 120.0;
    chase.notices.push(Notice::Airborne { altitude: 2.0, secs: 1.2 });
    chase.run(1.5);
    let air = chase.radio.first(Line::CollWorldAir).expect("airborne");
    assert_eq!(air.params.intensity, Some(Intensity::High));
    chase.snap.player.up_y = -1.0;
    chase.run(3.0);
    assert!(chase.radio.first(Line::CollWorldFlip).is_some());
}

#[test]
fn a_u_turn_is_remarked_on() {
    let mut chase = running(12);
    chase.snap.player.road_forward = Some(Vec3::Z);
    chase.snap.player.forward = Vec3::Z;
    chase.run(1.0);
    // The car swings round through the road's side over eight seconds (the detector wants twelve observation samples
    // between the first and the last) and then holds the reverse heading.
    for i in 0..80 {
        let angle = std::f32::consts::PI * (i as f32 + 1.0) / 80.0;
        chase.snap.player.forward = Vec3::new(angle.sin(), 0.0, angle.cos());
        chase.run(0.1);
    }
    chase.snap.player.forward = -Vec3::Z;
    chase.run(8.0);
    assert!(chase.radio.first(Line::SuspectUTurn).is_some(), "{:?}", chase.radio.lines());
}

#[test]
fn heavy_braking_on_the_road_is_remarked_on() {
    let mut chase = running(13);
    chase.snap.player.road_forward = Some(Vec3::Z);
    chase.snap.player.speed_mph = 100.0;
    chase.snap.player.brake = 1.0;
    chase.run(0.6);
    chase.snap.player.speed_mph = 20.0;
    chase.run(1.0);
    assert!(chase.radio.first(Line::SuspectBrake).is_some(), "{:?}", chase.radio.lines());
}

#[test]
fn passing_a_slow_cop_at_speed_is_spotted_by_that_cop() {
    let mut chase = running(14);
    chase.snap.cops = vec![CopInfo { speed_mph: 20.0, ..cop(10, 40.0) }];
    chase.snap.player.speed_mph = 100.0;
    chase.run(0.5);
    chase.notices.push(Notice::BlewByCop(10));
    chase.run(1.0);
    assert!(chase.radio.first(Line::Spotted).is_some());
}

#[test]
fn a_tunnel_makes_the_helicopter_warn_and_lose_sight() {
    let mut chase = running(15);
    chase.snap.heli = Some(HeliInfo { id: 99, active: true, los: true, distance: 80.0, health: 1.0, fuel_secs: None });
    chase.run(1.0);
    chase.notices.push(Notice::Tunnel(true));
    chase.run(1.0);
    let alert = chase.radio.first(Line::HeliHazardAlert).expect("hazard alert");
    assert_eq!(alert.params.hazard, Some(Hazard::ApproachingTunnel));
}

#[test]
fn a_gas_station_going_up_brings_a_call_for_the_fire_service() {
    let mut chase = running(16);
    chase.snap.cops[0].position = Vec3::new(10.0, 0.0, 0.0);
    chase.notices.push(Notice::GasStationExploded { by_player: true, at: Vec3::ZERO });
    chase.run(3.0);
    let call = chase.radio.first(Line::CallForEv).expect("call for ev");
    assert_eq!(call.params.ev, Some(Ev::GasStationFire));
    assert!(
        chase.radio.first(Line::CollisionWorld).map(|c| c.params.world_object) == Some(Some(WorldObject::GasStation))
    );
}

fn with_roadblock(seed: u64) -> Chase {
    let mut chase = running(seed);
    chase.snap.level.roadblocks = true;
    chase.snap.level.spikes = true;
    chase
}

#[test]
fn the_leader_calls_for_a_roadblock_and_dispatch_says_yes() {
    let mut chase = with_roadblock(20);
    chase.run(4.0);
    let call = chase.radio.first(Line::CallForRoadblock).expect("call for roadblock");
    assert_eq!(call.params.roadblock, Some(RoadblockKind::Spikes));
    let reply = chase.radio.first(Line::DispRoadblockReply).expect("dispatch reply");
    assert_eq!(reply.params.yes, Some(true));
    assert_eq!(reply.speaker, DISPATCH);
}

#[test]
fn a_disabled_roadblock_gets_a_no() {
    let mut chase = running(21);
    chase.snap.level.roadblocks = false;
    chase.snap.level.spikes = true;
    chase.run(4.0);
    let reply = chase.radio.first(Line::DispRoadblockReply).expect("dispatch reply");
    assert_eq!(reply.params.yes, Some(false));
    assert!(chase.radio.first(Line::NegativeRoadblockReply).is_some());
}

fn block(id: u32) -> RoadblockInfo {
    RoadblockInfo { id, visual: false, spike_strips: 0, cops_damaged: 0, cops_destroyed: 0, distance: 200.0 }
}

#[test]
fn a_roadblock_appearing_is_announced() {
    let mut chase = with_roadblock(22);
    chase.run(4.0);
    chase.snap.roadblock = Some(block(1));
    chase.run(4.0);
    let lines = chase.radio.lines();
    let warned = lines.contains(&Line::RoadblockWarning)
        || lines.contains(&Line::PursuitApproaching)
        || lines.contains(&Line::RoadblockApproach);
    assert!(warned, "{lines:?}");
}

#[test]
fn the_spike_strips_position_is_called_when_the_cars_are_placed() {
    let mut chase = with_roadblock(23);
    chase.run(4.0);
    chase.snap.roadblock = Some(RoadblockInfo { spike_strips: 1, ..block(1) });
    chase.notices.push(Notice::RoadblockPosition(-6));
    chase.run(4.0);
    let position = chase.radio.first(Line::RoadblockPosition).expect("spike position");
    assert_eq!(position.params.spike_position, Some(SpikePosition::Left));
    assert_eq!(position.speaker, 0, "a voiceless line");
}

#[test]
fn hitting_a_roadblock_with_sight_makes_a_cop_call_the_arrest() {
    let mut chase = with_roadblock(24);
    chase.run(4.0);
    chase.snap.roadblock = Some(RoadblockInfo { visual: true, ..block(1) });
    chase.run(2.0);
    chase.notices.push(Notice::RoadblockHit(RoadblockHit::Vehicle));
    chase.run(8.0);
    assert!(chase.radio.first(Line::Arrest).is_some(), "{:?}", chase.radio.lines());
}

#[test]
fn dodging_a_roadblock_ends_with_an_averted_line_and_the_next_roadblock() {
    let mut chase = with_roadblock(25);
    chase.run(4.0);
    chase.snap.roadblock = Some(block(1));
    chase.run(3.0);
    chase.notices.push(Notice::RoadblockDodged);
    chase.run(4.0);
    let lines = chase.radio.lines();
    let averted = lines.contains(&Line::RoadblockAverted)
        || lines.contains(&Line::ExtraRoadblockAverted)
        || lines.contains(&Line::DispSubRoadblock)
        || lines.contains(&Line::CallForSubRoadblock);
    assert!(averted, "{lines:?}");
}

#[test]
fn the_helicopter_joining_a_roadblock_says_so() {
    let mut chase = with_roadblock(26);
    chase.snap.heli = Some(HeliInfo { id: 99, active: true, los: true, distance: 80.0, health: 1.0, fuel_secs: None });
    chase.run(3.0);
    chase.notices.push(Notice::HeliJoinsRoadblock);
    chase.run(2.0);
    let line = chase.radio.first(Line::HeliSelfStrategy).expect("heli self strategy");
    assert_eq!(line.params.heli_strategy, Some(HeliStrategy::Roadblock));
}

fn free_roam() -> Chase {
    let mut chase = Chase::new(30);
    chase.snap.pursuit = PursuitInfo::default();
    chase.snap.cops.clear();
    chase.snap.level = Level {
        cops_enabled: true,
        roaming: true,
        cts_for_911: 100,
        hits_for_911: 3,
        lifetime_911: 90.0,
        ..Level::default()
    };
    chase
}

#[test]
fn smashing_enough_property_in_free_roam_gets_a_911_report() {
    let mut chase = free_roam();
    chase.run(2.0);
    assert_eq!(chase.radio.count(Line::Disp911Report), 0);
    chase.notices.push(Notice::Smashed { cost: 150 });
    chase.run(3.0);
    let report = chase.radio.first(Line::Disp911Report).expect("911 report");
    assert_eq!(report.speaker, DISPATCH);
    assert_eq!(report.params.encounter, Some(Encounter::First));
    assert_eq!(report.params.pursuit_type, Some(PursuitType::GenericSpeeder));
    assert_eq!(report.params.road, Some(5));
    assert!(report.params.direction.is_some());
}

#[test]
fn the_911_call_ends_with_more_details_and_the_cops_stay_away() {
    let mut chase = free_roam();
    chase.notices.push(Notice::Smashed { cost: 150 });
    chase.run(8.0);
    assert!(chase.radio.effects.contains(&Effect::Force911 { secs: 90.0 }));
    assert_eq!(chase.radio.count(Line::MoreDetails), 1);
    let lines = chase.radio.lines();
    assert!(lines.contains(&Line::VehicleDescription) || lines.contains(&Line::NoVehicleDescription), "{lines:?}");
    assert_eq!(chase.radio.count(Line::Disp911Report), 1, "once");
    assert!(chase.radio.first(Line::Unit911Reply).is_some());
}

#[test]
fn hard_traffic_hits_in_free_roam_also_get_a_911_report() {
    let mut chase = free_roam();
    for _ in 0..3 {
        chase.notices.push(Notice::PlayerHitTraffic { intensity: 0.8, semi: false });
        chase.run(0.2);
    }
    chase.run(3.0);
    assert_eq!(chase.radio.count(Line::Disp911Report), 1);
}

#[test]
fn a_911_report_in_a_hot_chase_world_is_a_possible_wanted() {
    let mut chase = free_roam();
    chase.snap.pursuit.heat = 3.0;
    chase.notices.push(Notice::Smashed { cost: 150 });
    chase.run(3.0);
    let report = chase.radio.first(Line::Disp911Report).expect("911 report");
    assert_eq!(report.params.pursuit_type, Some(PursuitType::PossibleWanted));
}
