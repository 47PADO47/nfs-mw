use super::*;

#[test]
fn an_empty_database_and_unknown_car_are_empty() {
    let data = VisualEffectsData::read(&Database::new(), "bmwm3gtr");
    assert!(data.collision.hit(vlt_hash("default")).is_none());
    assert!(data.collision.scrape(vlt_hash("default")).is_none());
    assert!(data.trail.is_none());
}

#[test]
fn nonfinite_and_invalid_lifetime_or_color_are_rejected() {
    let color = [0.6, 0.7, 0.8, 0.1];
    assert!(EmitterStyle::validated(color, 0.25, 0.25).is_some());
    for life in [0.0, -1.0, f32::NAN, f32::INFINITY] {
        assert!(EmitterStyle::validated(color, life, 0.0).is_none());
    }
    for variance in [-0.01, 0.3, f32::NAN, f32::INFINITY] {
        assert!(EmitterStyle::validated(color, 0.25, variance).is_none());
    }
    for value in [-0.01, 1.01, f32::NAN, f32::INFINITY] {
        assert!(EmitterStyle::validated([value, 0.0, 0.0, 1.0], 1.0, 0.0).is_none());
    }
}

#[test]
fn runtime_graph_resolves_parent_links_and_explicit_unsupported_material_blocks_fallback() {
    let db = super::synthetic::database("");
    let emitter = db.collection("fuelcell_emitter", "emsprk_line1").unwrap();
    assert_eq!(emitter.get_f32("Life"), Some(2.0));
    assert_eq!(emitter.get_f32("LifeVariance"), Some(0.1));
    let data = VisualEffectsData::read(&db, "SYNTHETIC_CAR");
    let hit = data.collision.hit(vlt_hash("asphalt")).unwrap();
    assert_eq!((hit.min, hit.max), (1.0, 30.0));
    assert_eq!(hit.styles[0].unwrap(), EmitterStyle { color: [0.1, 0.2, 0.3, 0.4], life: 2.0, life_variance: 0.1 });
    assert_eq!(hit.styles[1], hit.styles[0]);
    assert_eq!(data.collision.scrape(vlt_hash("default")).unwrap().min, 5.0);
    assert_eq!(data.trail, hit.styles[0]);
    for name in ["wood", "wood_child"] {
        assert!(data.collision.hit(vlt_hash(name)).is_none());
        assert!(data.collision.scrape(vlt_hash(name)).is_none());
    }
    assert_eq!(data.collision.hit(0), Some(hit));
    assert_eq!(data.collision.hit(vlt_hash("unknown_material")), Some(hit));
    assert_eq!(data.collision.hit(vlt_hash("null")), Some(hit), "native lookup does not visit null overrides");
    let unknown_override = VisualEffectsData::read(&super::synthetic::database("unknown_override"), "synthetic_car");
    assert!(unknown_override.collision.hit(0).is_none(), "world lookup resolves absent hashes to unknown");
}

#[test]
fn malformed_records_references_ranges_and_styles_fail_closed() {
    for broken in [
        "record_type",
        "selector",
        "effect",
        "group",
        "emitter",
        "xenon",
        "line",
        "missing",
        "range",
        "nan_range",
        "color",
        "life",
        "variance",
    ] {
        let data = VisualEffectsData::read(&super::synthetic::database(broken), "synthetic_car");
        assert!(data.collision.hit(vlt_hash("default")).is_none(), "{broken}");
        assert!(data.collision.hit(vlt_hash("asphalt")).is_none(), "{broken}");
        assert!(data.collision.hit(vlt_hash("wood")).is_none(), "{broken}");
    }
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn installed_bmw_visual_channels_resolve_supported_runtime_styles_and_exclude_wood() {
    let Some(path) = std::env::var_os(crate::game::SPEC.env_var) else { return };
    let dir = game_install::GameDir::open(std::path::PathBuf::from(path)).unwrap();
    let db = Database::open(&dir.read("GLOBAL/ATTRIBUTES.BIN").unwrap()).unwrap();
    let data = VisualEffectsData::read(&db, "bmwm3gtr");
    let hit = data.collision.hit(vlt_hash("default")).unwrap();
    let scrape = data.collision.scrape(vlt_hash("concrete")).unwrap();
    assert!(hit.max > hit.min && scrape.max > scrape.min);
    assert!(data.collision.hit(vlt_hash("wood")).is_none());
    assert!(data.collision.scrape(vlt_hash("wood")).is_none());
    assert_eq!(data.collision.hit(vlt_hash("asphalt")), Some(hit));
    assert_eq!(data.collision.hit(0), Some(hit));
    assert_eq!(data.collision.hit(vlt_hash("absent_world_material")), Some(hit));
    assert_eq!(data.collision.scrape(0), data.collision.scrape(vlt_hash("default")));
    for (i, name) in ["emsprk_line1", "emsprk_line2"].into_iter().enumerate() {
        let raw = db.collection("fuelcell_emitter", name).unwrap();
        assert_eq!(hit.styles[i], EmitterStyle::read(raw));
        assert!(hit.styles[i].is_some());
        assert_eq!(scrape.styles[i], hit.styles[i]);
    }
    assert_eq!(data.trail, EmitterStyle::read(db.collection("fuelcell_emitter", "trail3").unwrap()));
    assert!(data.trail.is_some());
}
