use super::*;
use blackbox_attrib::Value;
use nfsmw_data::car::physics::{DAMAGE_ZONES, PhysicsData};

#[test]
#[ignore = "requires installed cop damage records"]
fn installed_cop_health_tuning_maps_every_zone_and_inherited_record_without_name_fallback() {
    let dir = game_install::GameDir::open(std::env::var_os("NFSMW_GAME_DIR").expect("set NFSMW_GAME_DIR")).unwrap();
    let data = PhysicsData::load(&dir).unwrap();
    for name in [
        "copmidsize",
        "copghost",
        "copgto",
        "copgtoghost",
        "copsport",
        "copsportghost",
        "copsporthench",
        "copsuv",
        "copsuvl",
        "copsuvpatrol",
        "copcross",
    ] {
        let physics = data.car(name).unwrap();
        let spec = physics.events.as_ref().expect("effective installed damage behavior");
        let source = data
            .lineage(name)
            .into_iter()
            .find_map(|ancestor| {
                data.database().collection("pvehicle", &ancestor.to_ascii_lowercase())?.follow("damagespecs")
            })
            .expect("source damage collection, including unnamed sport variants");
        let health = CopDamage::for_model("deliberately_unknown_model_alias", Some(spec));
        assert_eq!(health.tuning.hit_points, source.get_f32("HIT_POINTS").unwrap());
        assert_eq!(health.tuning.threshold, source.get_f32("HP_THRESHOLD").unwrap());
        assert_eq!(health.tuning.suppress_dist, source.get_f32("SUPPRESS_DIST").unwrap());
        assert_eq!(health.tuning.strength, source.get_f32("FORCE").unwrap() * 1000.0);
        assert_eq!(health.shock_force, source.get_f32("SHOCK_FORCE").unwrap());
        assert_eq!(health.shock_time, source.get_f32("SHOCK_TIME").unwrap());
        for (zone, field) in DAMAGE_ZONES.into_iter().enumerate() {
            let Value::Raw { bytes, .. } = source.get(field).unwrap() else { panic!("source zone record") };
            let visual = f32::from_le_bytes(bytes[..4].try_into().unwrap());
            let hit_points = f32::from_le_bytes(bytes[4..8].try_into().unwrap());
            assert_eq!(spec.visual_scale[zone], visual, "{name} {field} visual scale");
            assert_eq!(health.scales[zone], hit_points, "{name} {field} independent health scale");
        }
        eprintln!(
            "{name}: damage={:08X}, HP={}, threshold={}, strength={}, suppress={}, visual={:?}, health={:?}",
            source.key(),
            spec.hit_points,
            spec.hit_point_threshold,
            spec.strength,
            spec.suppress_distance,
            spec.visual_scale,
            health.scales
        );
    }
}
