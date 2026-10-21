//! The crash timer and the time of the last ram, which the hard-braking remark reads.

use glam::Vec3;

use super::super::*;
use super::support::*;

fn running(seed: u64) -> Chase {
    let mut chase = Chase::new(seed);
    chase.snap.cops = vec![cop(10, 40.0), cop(11, 70.0)];
    chase.snap.player.road_forward = Some(Vec3::Z);
    chase.run(16.0);
    assert_eq!(chase.machine.focus(), Focus::Strategy);
    chase
}

fn brake_hard(chase: &mut Chase) {
    chase.snap.player.speed_mph = 100.0;
    chase.snap.player.brake = 1.0;
    chase.run(0.6);
    chase.snap.player.speed_mph = 20.0;
    chase.run(1.0);
}

#[test]
fn a_crash_against_traffic_holds_back_the_hard_braking_remark() {
    let mut chase = running(1);
    chase.notices.push(Notice::Crashed);
    chase.run(0.3);
    brake_hard(&mut chase);
    assert_eq!(chase.radio.count(Line::SuspectBrake), 0, "the stop followed a crash");
}

#[test]
fn a_ram_on_a_braking_cop_still_stamps_the_time_of_the_last_ram() {
    let mut chase = running(2);
    chase.notices.push(Notice::PlayerHitCop { cop: 10, impact: Impact::RearEnd, intensity: 0.2, deliberate: false });
    chase.run(0.3);
    brake_hard(&mut chase);
    assert_eq!(chase.radio.count(Line::SuspectBrake), 0, "the original stamps the ram before it asks whose car it was");
}

#[test]
fn a_hit_the_cop_caused_does_not_stamp_the_time_of_the_last_ram() {
    let mut chase = running(3);
    chase.notices.push(Notice::PlayerHitCop { cop: 10, impact: Impact::None, intensity: 0.2, deliberate: true });
    chase.run(0.3);
    brake_hard(&mut chase);
    assert_eq!(chase.radio.count(Line::SuspectBrake), 1);
}

#[test]
fn a_ram_with_both_noses_in_says_nothing_at_normal_strength_and_swears_at_high() {
    let mut chase = running(4);
    chase.notices.push(Notice::PlayerHitCop { cop: 10, impact: Impact::Other, intensity: 0.3, deliberate: true });
    chase.run(1.0);
    let said = [Line::RamRearEnd, Line::RamTBone, Line::RamHeadOn, Line::RamSideSwipe];
    assert!(said.iter().all(|&l| chase.radio.count(l) == 0));
    let mut chase = running(5);
    chase.notices.push(Notice::PlayerHitCop { cop: 10, impact: Impact::Other, intensity: 0.9, deliberate: true });
    chase.run(1.0);
    assert!(chase.radio.first(Line::InterruptViolent).is_some());
}
