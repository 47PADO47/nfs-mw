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
    let profile = EmitterStyle { color, life: 0.25, life_variance: 0.25, ..EmitterStyle::default() };
    assert!(profile.validated().is_some());
    for life in [0.0, -1.0, f32::NAN, f32::INFINITY] {
        assert!(EmitterStyle { life, ..profile }.validated().is_none());
    }
    for variance in [-0.01, 1.0, f32::NAN, f32::INFINITY] {
        assert!(EmitterStyle { life_variance: variance, ..profile }.validated().is_none());
    }
    for value in [-0.01, 1.01, f32::NAN, f32::INFINITY] {
        assert!(EmitterStyle { color: [value, 0.0, 0.0, 1.0], ..profile }.validated().is_none());
    }
}

#[test]
fn invalid_motion_volume_and_size_cannot_enter_the_particle_simulation() {
    let profile = EmitterStyle { color: [1.0; 4], life: 1.0, ..EmitterStyle::default() };
    for invalid in [
        EmitterStyle { velocity: [f32::NAN, 0.0, 0.0], ..profile },
        EmitterStyle { inherit: [0.0, f32::INFINITY, 0.0], ..profile },
        EmitterStyle { extent: [-1.0, 0.0, 0.0], ..profile },
        EmitterStyle { gravity: f32::INFINITY, ..profile },
        EmitterStyle { count: 1e6, ..profile },
        EmitterStyle { height: -1.0, ..profile },
        EmitterStyle { length_delta: f32::NAN, ..profile },
    ] {
        assert!(invalid.validated().is_none());
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
    assert_eq!(
        hit.styles[0].unwrap(),
        EmitterStyle { color: [0.1, 0.2, 0.3, 0.4], life: 2.0, life_variance: 0.1, ..EmitterStyle::default() }
    );
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

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn installed_pc_collision_profiles_and_their_textures_resolve_without_mod_assets() {
    let path = std::env::var_os(crate::game::SPEC.env_var).expect("set NFSMW_GAME_DIR");
    let dir = game_install::GameDir::open(std::path::PathBuf::from(path)).unwrap();
    let db = Database::open(&dir.read("GLOBAL/ATTRIBUTES.BIN").unwrap()).unwrap();
    let textures = crate::world::load_global_textures(&dir).unwrap();
    for car in ["bmwm3gtr", "cobaltss"] {
        let data = VisualEffectsData::read(&db, car);
        let hit = data.collision.pc_hit(vlt_hash("default")).unwrap();
        let scrape = data.collision.pc_scrape(vlt_hash("default")).unwrap();
        assert_eq!(hit.emitters.iter().flatten().count(), 3);
        assert_eq!(scrape.emitters.iter().flatten().count(), 4);
        assert!(scrape.emitters.iter().flatten().any(|e| e.grid == 2 && e.fps == 20));
        for e in hit.emitters.iter().chain(&scrape.emitters).flatten() {
            let texture =
                textures.textures.iter().find(|t| t.name_hash == e.texture).expect("ordinary PC sprite in base assets");
            assert_eq!(texture.alpha_blend, 2);
            assert!(blackbox_tpk::decode_rgba8(texture).is_some());
        }
        let wood = data.collision.pc_hit(vlt_hash("wood")).unwrap();
        assert!(wood.emitters.iter().flatten().all(|e| !hit.emitters.iter().flatten().any(|spark| spark.key == e.key)));
    }
}
