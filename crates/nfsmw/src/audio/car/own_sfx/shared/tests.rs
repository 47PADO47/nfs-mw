use super::*;

#[test]
fn a_blow_is_light_medium_or_heavy_by_its_strength() {
    assert_eq!([0.0, 0.2, 0.3, 0.5, 0.7, 0.8, 1.0].map(impact_class), [0, 0, 1, 1, 1, 2, 2]);
    assert_eq!(impact_class(7.0), 2);
    assert_eq!(impact_class(-1.0), 0);
}

#[test]
fn a_harder_blow_and_a_harder_scrape_are_louder_and_none_is_silent() {
    assert!(impact_volume(0.0) > 0.5 && impact_volume(1.0) <= 1.0);
    assert!(impact_volume(0.9) > impact_volume(0.1));
    assert!(scrape_volume(0.0) > 0.3 && scrape_volume(1.0) <= 1.0);
    assert!(scrape_volume(0.9) > scrape_volume(0.1));
}

#[test]
fn a_scrape_starts_a_one_shot_and_the_next_waits_until_most_of_it_has_played() {
    let mut clock = ScrapeClock::default();
    assert!(matches!(clock.step(0.5, 0.01), ScrapeStep::Trigger { .. }));
    clock.started(0.8);
    let mut waited = 0.0;
    loop {
        match clock.step(0.5, 0.01) {
            ScrapeStep::Trigger { .. } => break,
            step => assert_eq!(step, ScrapeStep::Idle),
        }
        waited += 0.01;
        assert!(waited < 1.0, "the next one-shot never started");
    }
    assert!((0.6..0.75).contains(&waited), "waited {waited}");
}

#[test]
fn a_scrape_that_stops_is_released_once_and_a_silent_clock_does_nothing() {
    let mut clock = ScrapeClock::default();
    assert_eq!(clock.step(0.0, 1.0), ScrapeStep::Idle, "nothing was playing");
    assert!(matches!(clock.step(0.4, 0.01), ScrapeStep::Trigger { .. }));
    assert_eq!(clock.step(0.0, 0.1), ScrapeStep::Idle, "a short break is not the end");
    assert_eq!(clock.step(0.0, 0.1), ScrapeStep::Release);
    assert_eq!(clock.step(0.0, 0.1), ScrapeStep::Idle, "released once");
    assert_eq!(clock.step(f32::NAN, 0.1), ScrapeStep::Idle);
}

fn hit(kind: EventKind, smackable: bool) -> CarEvent {
    CarEvent {
        sound_profile: 0,
        vehicle: None,
        audio_zone: None,
        kind,
        surface: 0,
        magnitude: 0.5,
        front: true,
        smackable,
        position: glam::Vec3::ZERO,
    }
}

fn data() -> StaticSoundData {
    use kira::Frame;
    use kira::sound::static_sound::StaticSoundSettings;
    StaticSoundData {
        sample_rate: 48_000,
        frames: std::sync::Arc::from(vec![Frame::from_mono(0.5); 4]),
        settings: StaticSoundSettings::new(),
        slice: None,
    }
}

#[test]
fn only_body_hits_that_are_not_smackable_are_taken_and_only_when_the_car_has_recordings() {
    let none = SharedSfx::new([vec![], vec![], vec![]], vec![], 1);
    assert!(none.is_empty() && !none.has_scrapes());
    assert!(!none.takes(&hit(EventKind::HitWorld, false)));
    let sfx = SharedSfx::new([vec![], vec![data()], vec![]], vec![data()], 1);
    assert!(!sfx.is_empty() && sfx.has_scrapes());
    assert!(sfx.takes(&hit(EventKind::HitWorld, false)) && sfx.takes(&hit(EventKind::HitObject, false)));
    assert!(!sfx.takes(&hit(EventKind::HitObject, true)), "a cone is not a car crash");
    assert!(!sfx.takes(&hit(EventKind::HitGround, false)));
    assert!(!sfx.takes(&hit(EventKind::BottomOut, false)));
    assert!(!sfx.takes(&hit(EventKind::Detached, false)));
}

#[test]
fn a_class_without_files_plays_the_nearest_class_that_has_some() {
    let sfx = SharedSfx::new([vec![], vec![data()], vec![]], vec![], 1);
    assert_eq!([0, 1, 2].map(|class| sfx.class_with_files(class)), [Some(1), Some(1), Some(1)]);
    let sfx = SharedSfx::new([vec![data()], vec![], vec![data()]], vec![], 1);
    assert_eq!([0, 1, 2].map(|class| sfx.class_with_files(class)), [Some(0), Some(2), Some(2)]);
    assert_eq!(SharedSfx::new([vec![], vec![], vec![]], vec![], 1).class_with_files(1), None);
}
