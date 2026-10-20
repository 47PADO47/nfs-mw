use super::*;

mod installed;

fn link(surface: u32, name: &str) -> Link {
    Link {
        kind: EventKind::HitWorld,
        surface,
        minimum: 2.0,
        maximum: 8.0,
        audio_curve: [0.0, 1.0, 0.0, 0.0],
        audio_cull_distance: 30.0,
        sounds: vec![(CollisionZone::Default, Some(ImpactSound { name: name.into(), ..Default::default() }))],
    }
}

#[test]
fn authored_surface_ancestry_precedes_default_and_missing_rules_are_silent() {
    let mut sounds = CollisionSounds {
        links: vec![link(1, "default"), link(10, "parent"), link(11, "child")],
        parents: HashMap::from([(11, 10), (12, 10)]),
    };
    assert_eq!(sounds.pick(EventKind::HitWorld, 11, 1, false).unwrap().name, "child");
    assert_eq!(sounds.pick(EventKind::HitWorld, 12, 1, false).unwrap().name, "parent");
    assert_eq!(sounds.pick(EventKind::HitWorld, 99, 1, false).unwrap().name, "default");
    sounds.links.remove(0);
    assert!(sounds.pick(EventKind::HitWorld, 99, 1, false).is_none());
    assert!(sounds.pick(EventKind::HitGround, 11, 1, false).is_none());
    sounds.parents.insert(12, 13);
    sounds.parents.insert(13, 12);
    assert!(sounds.pick(EventKind::HitWorld, 12, 1, false).is_none());
}

#[test]
fn selected_rule_uses_its_effective_velocity_change_range() {
    let sound = ImpactSound::default();
    let rule = CollisionRule {
        sound: &sound,
        minimum: 2.0,
        maximum: 8.0,
        audio_curve: [0.0, 1.0, 0.0, 0.0],
        audio_cull_distance: 30.0,
    };
    assert_eq!(rule.magnitude(1.0), None);
    assert_eq!(rule.magnitude(2.0), Some(0.0));
    assert_eq!(rule.magnitude(5.0), Some(0.5));
    assert_eq!(rule.magnitude(30.0), Some(1.0));
    for value in [f32::NAN, f32::INFINITY] {
        assert_eq!(rule.magnitude(value), None);
    }
    let tiny = CollisionRule { minimum: 0.0, maximum: 0.0000005, ..rule };
    assert_eq!(tiny.magnitude(0.0000002), None);
}

#[test]
fn signed_audio_zones_keep_strict_boundaries_and_forward_precedence() {
    use CollisionZone::*;
    use glam::Vec3;
    for (normal, expected) in [
        (Vec3::NEG_Z, Front),
        (Vec3::Z, Rear),
        (Vec3::NEG_Y, Top),
        (Vec3::Y, Bottom),
        (Vec3::X, Side),
        (Vec3::new(0.0, 0.0, -0.707), Side),
        (Vec3::new(0.0, 0.0, 0.707), Side),
        (Vec3::new(0.0, -0.707, 0.0), Side),
        (Vec3::new(0.0, 0.707, 0.0), Side),
        (Vec3::new(0.0, -0.8, -0.8), Front),
    ] {
        assert_eq!(CollisionZone::from_local_normal(normal), expected);
    }
}

#[test]
fn exact_audio_fields_override_description_and_only_default_is_a_fallback() {
    let mut selected = link(10, "ordinary");
    selected.sounds.push((
        CollisionZone::Rear,
        Some(ImpactSound { name: "rear-without-front-description".into(), ..Default::default() }),
    ));
    let mut sounds = CollisionSounds { links: vec![selected], parents: HashMap::new() };
    let pick = |sounds: &CollisionSounds, zone| {
        sounds.rule_for_zone(EventKind::HitWorld, 10, 1, zone).map(|r| r.sound.name.clone())
    };
    assert_eq!(pick(&sounds, CollisionZone::Rear).as_deref(), Some("rear-without-front-description"));
    assert_eq!(pick(&sounds, CollisionZone::Top).as_deref(), Some("ordinary"));
    sounds.links[0].sounds.remove(0);
    assert_eq!(pick(&sounds, CollisionZone::Front), None, "an unrelated reference is not a fallback");
    sounds.links[0].sounds.push((CollisionZone::Default, Some(ImpactSound::default())));
    sounds.links[0].sounds.push((CollisionZone::Top, None));
    assert_eq!(pick(&sounds, CollisionZone::Top), None, "a present unresolved reference stays silent");
}

#[test]
fn material_effect_without_audio_remains_silent_instead_of_using_default_material() {
    let mut silent = link(10, "silent");
    silent.sounds.clear();
    let sounds =
        CollisionSounds { links: vec![link(1, "default material"), silent], parents: HashMap::from([(11, 10)]) };
    assert!(sounds.rule_for_zone(EventKind::HitWorld, 11, 1, CollisionZone::Default).is_none());
}

#[test]
fn authored_audio_curve_maps_physical_ramps_once_before_selection() {
    let sound = ImpactSound::default();
    for (curve, expected) in [
        ([0.0, 1.0, 0.0, 0.0], 0.5),
        ([0.0, 2.0, -1.0, 0.0], 0.75),
        ([0.0, 1.8, -0.8, 0.0], 0.7),
        ([-0.25, 1.0, 0.25, 0.0], 0.3125),
        ([0.05, 2.0, 0.0, 0.0], 1.0),
        ([0.15, 2.5, 0.0, 0.0], 1.0),
    ] {
        let rule =
            CollisionRule { sound: &sound, minimum: 2.0, maximum: 8.0, audio_curve: curve, audio_cull_distance: 30.0 };
        assert_eq!(rule.magnitude(5.0), Some(0.5));
        assert!((rule.audio_magnitude(0.5) - expected).abs() < 1e-6);
        assert_eq!(rule.audio_magnitude(f32::NAN), 0.0);
    }
}

#[test]
fn source_cull_boundary_differs_for_one_shots_and_continuous_audio() {
    let sound = ImpactSound::default();
    let rule = CollisionRule {
        sound: &sound,
        minimum: 0.0,
        maximum: 1.0,
        audio_curve: [0.0, 1.0, 0.0, 0.0],
        audio_cull_distance: 30.0,
    };
    assert!(rule.audible_at(30.0, false));
    assert!(!rule.audible_at(30.0, true));
    assert!(!rule.audible_at(30.001, false));
    assert!(!rule.audible_at(f32::NAN, false));
    let distant = CollisionRule { audio_cull_distance: 200.0, ..rule };
    assert!(distant.audible_at(100.0, false));
    assert!(!distant.audible_at(100.001, false));
}

#[test]
fn silent_detachment_accepts_the_physical_effect_and_suppresses_object_fallback() {
    let mut detached = link(1, "particle-only detachment");
    detached.kind = EventKind::Detached;
    detached.sounds.clear();
    let sounds = CollisionSounds { links: vec![detached], parents: HashMap::new() };
    assert!(sounds.accepts_hit(EventKind::Detached, 1, 1, 5.0));
    assert!(!sounds.accepts_hit(EventKind::Detached, 1, 1, 2.0));
    assert!(sounds.rule_for_zone(EventKind::Detached, 1, 1, CollisionZone::Default).is_none());
}
