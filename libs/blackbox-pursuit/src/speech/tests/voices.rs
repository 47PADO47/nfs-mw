//! Who gets which voice: ordinary cops only ever speak with a ground voice (primary or secondary), never the
//! helicopter pilot's, the dispatch's or the cross car's, also after a helicopter has left the chase.

use super::super::actors::{Roster, VoiceKind};
use super::super::state::Core;
use super::super::*;
use super::support::*;

fn heli(id: u32) -> HeliInfo {
    HeliInfo { id, active: true, los: true, distance: 80.0, health: 1.0, fuel_secs: Some(100.0) }
}

#[test]
fn a_departed_helicopter_never_lends_its_voice_to_a_car() {
    for seed in 0..8 {
        let mut chase = Chase::new(seed);
        chase.snap.cops = vec![cop(10, 40.0)];
        chase.snap.heli = Some(heli(99));
        chase.run(6.0);
        assert!(chase.machine.cops().iter().any(|a| a.heli && a.voice == HELI), "seed {seed}: the pilot joined");
        chase.snap.heli = None;
        chase.run(1.0);
        chase.snap.cops = (10..17).map(|id| cop(id, 30.0 + id as f32)).collect();
        chase.run(4.0);
        for actor in chase.machine.cops() {
            assert!(!actor.heli, "seed {seed}: the helicopter has gone");
            assert!((3..=8).contains(&actor.voice), "seed {seed}: car {} speaks with voice {}", actor.id, actor.voice);
        }
    }
}

#[test]
fn the_voice_pool_hands_out_only_ground_voices_to_ordinary_cars() {
    let layout = layout();
    let mut roster = Roster::new(&layout, &mut Rng::new(3));
    roster.free_voice(&layout, HELI);
    roster.free_voice(&layout, DISPATCH);
    let mut given = Vec::new();
    while let Some(voice) = roster.take_voice(&layout, VoiceKind::Any) {
        given.push(voice);
    }
    given.sort_unstable();
    assert_eq!(given, vec![3, 4, 5, 6, 7, 8], "the cross voice stays for the cross car");
    assert_eq!(roster.take_voice(&layout, VoiceKind::Cross), Some(9));
}

#[test]
fn a_random_cop_voice_is_a_ground_voice() {
    let (layout, tune, snap, radio) = (layout(), tune(), Snapshot::default(), Radio::new(1.0));
    let (mut core, mut out, mut rng) = (Core::new(), Output::default(), Rng::new(11));
    let mut roster = Roster::new(&layout, &mut rng);
    let mut cx = Cx {
        tune: &tune,
        layout: &layout,
        snap: &snap,
        chan: &radio,
        now: 0.0,
        core: &mut core,
        roster: &mut roster,
        rng: &mut rng,
        out: &mut out,
    };
    let mut heard = Vec::new();
    for _ in 0..200 {
        heard.push(cx.misc_bailout(0));
        cx.misc_lost_suspect(0);
        heard.push(cx.random_ground_voice());
        heard.push(cx.more_details());
    }
    heard.extend(cx.out.requests.iter().map(|r| r.speaker));
    assert!(heard.iter().all(|v| (3..=8).contains(v)), "{heard:?}");
    assert!(heard.contains(&3) && heard.contains(&8), "the whole range is used");
}
