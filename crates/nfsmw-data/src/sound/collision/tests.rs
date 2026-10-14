use super::*;

fn link(surface: u32, name: &str) -> Link {
    Link {
        kind: EventKind::HitWorld,
        surface,
        minimum: 2.0,
        maximum: 8.0,
        sounds: vec![ImpactSound { name: name.into(), ..Default::default() }],
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
    let rule = CollisionRule { sound: &sound, minimum: 2.0, maximum: 8.0 };
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
