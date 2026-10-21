use super::*;

#[test]
#[ignore = "requires installed attributes; set NFSMW_GAME_DIR"]
fn installed_collision_audio_zones_follow_exact_inherited_references() {
    let dir = game_install::GameDir::open(std::env::var("NFSMW_GAME_DIR").unwrap()).unwrap();
    let db = crate::shared::attributes(&dir).unwrap();
    let mut records = 0;
    let mut checks = 0;
    let mut inherited = 0;
    let mut directional = 0;
    let default = vlt_hash("default");
    for collection in db.collections_of("pvehicle").chain(db.collections_of("smackable")) {
        let sounds = CollisionSounds::from_collection(&db, collection);
        let mut seen = std::collections::HashSet::new();
        for kind in EventKind::ALL {
            let Some(Value::Array(items)) = collection.get(kind.field()) else { continue };
            for item in items {
                let Value::Raw { bytes, .. } = item else { continue };
                let surface = word(bytes, 4);
                if !seen.insert((kind, surface)) {
                    continue;
                }
                let Some(wrapper) = db.collection_by_key(word(bytes, 12), word(bytes, 16)) else { continue };
                records += 1;
                for zone in CollisionZone::ALL {
                    let reference = wrapper
                        .get(zone.field())
                        .and_then(Value::as_ref_spec)
                        .filter(|r| r.class != 0 && r.collection != 0);
                    let reference = reference.or_else(|| wrapper.get("AudioFX_DEFAULT").and_then(Value::as_ref_spec));
                    let expected = reference
                        .filter(|r| [vlt_hash("audioimpact"), vlt_hash("audioscrape")].contains(&r.class))
                        .and_then(|r| db.resolve(r))
                        .map(|c| c.key());
                    let actual = sounds.rule_for_zone(kind, surface, default, zone).map(|rule| rule.sound.key);
                    assert_eq!(
                        actual,
                        expected,
                        "owner={:?}, action={kind:?}, zone={zone:?}, effect={:?}",
                        collection.name(),
                        wrapper.name()
                    );
                    if let Some(rule) = sounds.rule_for_zone(kind, surface, default, zone) {
                        assert_eq!(
                            rule.audio_curve,
                            wrapper.get_vector4("AudioQuadratic").unwrap_or([0.0, 1.0, 0.0, 0.0])
                        );
                        assert_eq!(rule.audio_cull_distance, wrapper.get_f32("AudioCullDist").unwrap_or(0.0));
                    }
                    checks += 1;
                    if zone != CollisionZone::Default && wrapper.get(zone.field()).is_some() {
                        directional += 1;
                        inherited += usize::from(wrapper.get_own(zone.field()).is_none());
                    }
                }
            }
        }
    }
    assert!(records > 0 && checks >= records * CollisionZone::ALL.len());
    assert!(inherited > 0 && directional > 0, "installed fixtures must cover inherited directional sounds");
    println!("collision links={records}, zone checks={checks}, directional={directional}, inherited={inherited}");
}
