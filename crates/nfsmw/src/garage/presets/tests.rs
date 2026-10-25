use super::*;
use nfsmw_data::car::{customization, physics::performance::CategoryOptions};

fn fixture() -> (Model, Selection) {
    let mut car = crate::garage::Car { folder: "RACER".into(), paintable: true, ..Default::default() };
    car.performance.categories =
        [CategoryOptions { max_level: 4, junkman: true, stock_present: true, ..Default::default() }; 7];
    let mut catalog = customization::Catalog::default();
    catalog.choices[2].push(customization::Choice {
        hash: 123,
        label: "Hood".into(),
        language_hash: None,
        carbon: true,
    });
    for slot in [5, 6] {
        catalog.choices[slot].push(customization::Choice {
            hash: 200 + slot as u32,
            label: "Appearance".into(),
            language_hash: None,
            carbon: false,
        });
    }
    for slot in [4, 7, 8] {
        catalog.choices[slot].push(customization::Choice {
            hash: 300 + slot as u32,
            label: "New appearance".into(),
            language_hash: None,
            carbon: false,
        });
    }
    catalog.paintable_wheels.insert(304);
    catalog.gauge_skins = vec![0, 3];
    car.parts = Some(catalog.into());
    let mut selection = Selection {
        car: car.folder.clone(),
        color: Some([1, 100, 230]),
        performance: PerformancePackage { levels: [3; 7], junkman: 127 },
        ..Default::default()
    };
    selection.gauges = crate::garage::GaugeSelection {
        skin: 3,
        backing: Some([10, 20, 30]),
        needle: None,
        digits: Some([200, 100, 50]),
        ..Default::default()
    };
    selection.parts.vinyl_colors = [Some([40, 50, 60]), None, Some([70, 80, 90])];
    selection.tuning.profiles = [[-5; 7], [0; 7], [5; 7]];
    selection.tuning.active = 2;
    selection.parts.parts[4] = Some(304);
    selection.parts.parts[7] = Some(307);
    selection.parts.parts[8] = Some(308);
    selection.parts.rim_color = Some([20, 40, 200]);
    selection.parts.parts[2] = Some(123);
    selection.parts.parts[5] = Some(205);
    selection.parts.parts[6] = Some(206);
    (Model { cars: vec![car], ..Default::default() }, selection)
}

#[test]
fn a_car_preset_round_trips_all_fields_including_stock_parts_and_paint() {
    let (model, mut selection) = fixture();
    assert_eq!(decode(&encode(&selection).unwrap(), &model).unwrap(), selection);
    selection.color = None;
    selection.parts = Default::default();
    assert_eq!(decode(&encode(&selection).unwrap(), &model).unwrap(), selection);
}

#[test]
fn exhaust_presets_round_trip_reject_invalid_ranges_and_accept_legacy_setups() {
    let (model, mut selection) = fixture();
    selection.exhaust = crate::garage::ExhaustSelection::aggressive();
    selection.exhaust.root = [20, 255, 90];
    selection.exhaust.tail = [190, 30, 240];
    let text = encode(&selection).unwrap();
    selection.exhaust.set_style(Default::default());
    assert_eq!(decode(&text, &model).unwrap(), selection);
    let mut legacy: toml::Value = toml::from_str(&text).unwrap();
    legacy.as_table_mut().unwrap().remove("exhaust");
    assert_eq!(decode(&toml::to_string(&legacy).unwrap(), &model).unwrap().exhaust, Default::default());
    selection.exhaust.density = 999;
    assert!(encode(&selection).is_err());
    let mut invalid: toml::Value = toml::from_str(&text).unwrap();
    invalid["exhaust"]["density"] = toml::Value::Integer(999);
    assert!(decode(&toml::to_string(&invalid).unwrap(), &model).is_err());
}

#[test]
fn unavailable_equipment_bad_tuning_and_future_schemas_are_rejected() {
    let (mut model, selection) = fixture();
    let text = encode(&selection).unwrap();
    for bad in [
        text.replace("version = 1", "version = 99"),
        text.replace("active_tuning = 2", "active_tuning = 9"),
        text.replace("hash = 123", "hash = 999"),
        text.replace("car = \"RACER\"", "car = \"MISSING\""),
    ] {
        assert!(decode(&bad, &model).is_err());
    }
    model.cars[0].performance.categories[0].max_level = 2;
    assert!(decode(&text, &model).is_err());
    assert!(model.draft.car.is_empty() && model.applied.car.is_empty());
}

#[test]
fn legacy_presets_default_new_appearance_fields() {
    let (model, _) = fixture();
    let text = "version = 1\ncar = 'RACER'\nlevels = [0,0,0,0,0,0,0]\njunkman = 0\ntuning = [[0,0,0,0,0,0,0],[0,0,0,0,0,0,0],[0,0,0,0,0,0,0]]\nactive_tuning = 0\nparts = []\n";
    let restored = decode(text, &model).unwrap();
    assert_eq!(restored.gauges, Default::default());
    assert!(restored.parts.is_stock());
}

#[test]
fn sparse_native_color_identities_roundtrip_one_channel_without_toml_nulls() {
    let (mut model, mut selection) = fixture();
    let catalog = model.cars[0].parts.as_mut().unwrap();
    catalog.color_palettes[0] = vec![
        customization::ColorChoice { hash: 701, rgb: [40, 50, 60], upgrade_level: 1 },
        customization::ColorChoice { hash: 702, rgb: [40, 50, 60], upgrade_level: 2 },
    ];
    catalog.color_palettes[3] = vec![customization::ColorChoice { hash: 801, rgb: [10, 20, 30], upgrade_level: 1 }];
    selection.parts.vinyl_color_parts = [Some(702), None, None];
    selection.gauges.color_parts = [Some(801), None, None];
    let encoded = encode(&selection).unwrap();
    assert!(!encoded.contains("null"));
    let decoded = decode(&encoded, &model).unwrap();
    assert_eq!(decoded, selection);
    assert_eq!(decoded.parts.vinyl_color_parts, [Some(702), None, None]);
    assert_eq!(decoded.gauges.color_parts, [Some(801), None, None]);
    let bad = encoded.replace("hash = 702", "hash = 999");
    assert!(decode(&bad, &model).is_err(), "a native palette identity must belong to its channel");
}
