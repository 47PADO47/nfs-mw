use super::*;

#[test]
fn the_same_seed_always_gives_the_same_numbers_and_they_stay_in_range() {
    let (mut a, mut b) = (Rng::new(7), Rng::new(7));
    for _ in 0..200 {
        let x = a.unit();
        assert_eq!(x, b.unit());
        assert!((0.0..1.0).contains(&x));
    }
    let mut rng = Rng::new(0);
    assert!((0..500).all(|_| rng.below(3) < 3));
    assert_eq!(rng.below(0), 0);
    assert_eq!(rng.below(1), 0);
}

#[test]
fn the_turbo_spools_up_with_throttle_and_revs_and_winds_down_when_lifted() {
    let mut spool = 0.0;
    for _ in 0..300 {
        spool = spool_step(spool, 1.0, 1.0, 0.01);
    }
    assert!(spool > 0.9, "three seconds at full throttle and revs: {spool}");
    let low_revs = (0..300).fold(0.0, |s, _| spool_step(s, 1.0, 0.0, 0.01));
    assert!(low_revs < 0.3, "no boost at idle speed: {low_revs}");
    let lifted = (0..100).fold(spool, |s, _| spool_step(s, 0.0, 1.0, 0.01));
    assert!(lifted < 0.25 * spool, "a second off the throttle: {lifted}");
    assert!((0.0..=1.0).contains(&spool_step(2.0, 5.0, 9.0, 1.0)));
}

#[test]
fn the_spool_does_not_depend_on_how_the_time_is_cut() {
    let one = spool_step(0.1, 1.0, 0.8, 1.0);
    let many = (0..100).fold(0.1, |s, _| spool_step(s, 1.0, 0.8, 0.01));
    assert!((one - many).abs() < 1e-3, "{one} against {many}");
    assert_eq!(spool_step(0.4, 1.0, 1.0, f32::NAN), 0.4);
    assert_eq!(spool_step(0.4, 1.0, 1.0, -1.0), 0.4);
}

#[test]
fn the_whistle_is_silent_when_stopped_and_gets_louder_and_higher_with_the_spool() {
    for layers in [1, 2, 3] {
        assert!(whine_mix(layers, 0.0).iter().all(|layer| layer.gain == 0.0), "{layers} layers");
        let (half, full) = (whine_mix(layers, 0.5), whine_mix(layers, 1.0));
        let power = |mix: &[WhineMix]| mix.iter().map(|layer| layer.gain * layer.gain).sum::<f32>().sqrt();
        assert!(power(&full) > power(&half), "{layers} layers");
        assert!(full[0].rate > half[0].rate && half[0].rate > whine_mix(layers, 0.0)[0].rate);
        assert!(power(&full) <= WHINE_LEVEL + 1e-5);
    }
    assert!(whine_mix(0, 1.0).is_empty());
}

#[test]
fn whistle_loops_fade_into_each_other_along_the_spool_with_constant_power() {
    for spool in [0.1, 0.3, 0.5, 0.7, 0.95] {
        let mix = whine_mix(3, spool);
        let power = mix.iter().map(|layer| layer.gain * layer.gain).sum::<f32>();
        let total = spool.powf(1.5) * WHINE_LEVEL;
        assert!((power - total * total).abs() < 1e-5, "{spool}: {mix:?}");
        assert!(mix.iter().filter(|layer| layer.gain > 0.0).count() <= 2, "{spool}: only neighbours blend");
    }
    let first = whine_mix(3, 0.25);
    assert!(first[0].gain > 0.0 && first[2].gain == 0.0);
    let last = whine_mix(3, 1.0);
    assert!(last[2].gain > 0.0 && last[0].gain == 0.0);
}

#[test]
fn the_turbo_vents_only_with_spool() {
    let mut rng = Rng::new(3);
    assert_eq!(lift_off(0.1, 1.0, &mut rng).blowoff, None);
    let vented = lift_off(0.9, 1.0, &mut rng).blowoff.unwrap();
    let quiet = lift_off(0.4, 1.0, &mut rng).blowoff.unwrap();
    assert!(vented > quiet && vented <= 1.0 && quiet > 0.5);
}

#[test]
fn a_backfire_is_likelier_at_high_revs_and_is_followed_by_pops() {
    let backfires = |rpm_pct: f32| {
        let mut rng = Rng::new(11);
        (0..4000).filter(|_| lift_off(0.0, rpm_pct, &mut rng).backfire.is_some()).count() as f32 / 4000.0
    };
    assert!((backfires(0.0) - 0.2).abs() < 0.04, "{}", backfires(0.0));
    assert!((backfires(1.0) - 0.7).abs() < 0.04, "{}", backfires(1.0));
    let mut rng = Rng::new(5);
    for _ in 0..500 {
        let lift = lift_off(0.0, 0.8, &mut rng);
        if lift.backfire.is_some() {
            assert!((2..=4).contains(&lift.pops.len()), "{lift:?}");
        }
        assert!(lift.pops.len() <= 4);
        assert!(lift.pops.windows(2).all(|pair| pair[0].delay < pair[1].delay), "{lift:?}");
        assert!(lift.pops.iter().all(|pop| pop.delay >= 0.12 && (0.4..=0.8).contains(&pop.volume)), "{lift:?}");
        assert!(lift.backfire.is_none_or(|volume| (0.5..=1.0).contains(&volume)));
    }
}
