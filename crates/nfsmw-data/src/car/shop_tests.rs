use super::*;

fn raw(level: i32, cost: f32) -> Value {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&level.to_le_bytes());
    bytes.extend_from_slice(&1.0f32.to_le_bytes());
    bytes.extend_from_slice(&cost.to_le_bytes());
    Value::Raw { type_key: 0, bytes }
}

#[test]
fn price_records_truncate_cost_but_reject_nonfinite_and_negative_values() {
    assert_eq!(decode_price(&raw(3, 2500.75)).unwrap(), Price { level: 3, cost: 2500 });
    assert_eq!(decode_price(&raw(2, 0.0)).unwrap().cost, 0);
    assert!(decode_price(&raw(-1, 1.0)).is_err());
    assert!(decode_price(&raw(1, f32::NAN)).is_err());
    assert!(decode_price(&raw(1, f32::INFINITY)).is_err());
    assert!(decode_price(&raw(1, -1.0)).is_err());
    assert!(decode_price(&raw(1, u32::MAX as f32)).is_err());
}

#[test]
fn price_lookup_distinguishes_missing_and_authored_zero_cost() {
    let table = Prices { entries: HashMap::from([(8, vec![Price { level: 2, cost: 0 }])]) };
    assert_eq!(table.cost(8, 2).unwrap(), 0);
    assert!(table.cost(8, 3).is_err());
    assert!(table.cost(8, 0).is_err());
    assert_eq!(table.equipment_cost(8, 0).unwrap(), 0);
}

#[test]
fn stock_stays_zero_and_short_upgrade_ranges_map_to_global_tiers() {
    assert_eq!(global_level(PerformanceCategory::Engine, 2, 0).unwrap(), 0);
    assert_eq!(global_level(PerformanceCategory::Engine, 2, 1).unwrap(), 3);
    assert_eq!(global_level(PerformanceCategory::Engine, 2, 2).unwrap(), 4);
    assert_eq!(global_level(PerformanceCategory::Nitrous, 3, 1).unwrap(), 1);
    assert!(global_level(PerformanceCategory::Engine, 2, 3).is_err());
    assert!(global_level(PerformanceCategory::Tires, 4, 2).is_err());
}

#[test]
#[ignore = "needs the installed game"]
fn installed_garage_prices_cover_every_performance_and_body_category() {
    let folder = std::env::var_os("NFSMW_GAME_DIR").expect("NFSMW_GAME_DIR must select an install");
    let dir = GameDir::open(folder).unwrap();
    let table = Prices::load(&dir).unwrap();
    for category in PerformanceCategory::ALL {
        assert!(table.cost(performance_entity(category), 1).is_ok(), "{}", category.label());
    }
    for category in Category::MODELS {
        assert!(table.cost(visual_entity(category), 1).is_ok(), "{}", category.label());
    }
}
