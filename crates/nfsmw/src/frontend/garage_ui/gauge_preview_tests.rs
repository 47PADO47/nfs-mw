use super::*;
use blackbox_feng::package::{ObjectData, ObjectDef, ObjectKind, Package, StringDef};
use blackbox_feng::{NodeKind, ObjectRef};

fn setup() -> (Runtime, PackageId) {
    let mut names = vec![FACE, FILL, NEEDLE, TURBO_FACE, TURBO_NEEDLE, TURBO_GROUP];
    names.extend(DIGITS);
    let objects = names
        .into_iter()
        .enumerate()
        .map(|(i, name_hash)| {
            let kind = match name_hash {
                TURBO_GROUP => ObjectKind::Group,
                name if DIGITS.contains(&name) => ObjectKind::String,
                _ => ObjectKind::Image,
            };
            let mut words = vec![0; 17];
            words[..4].copy_from_slice(&[30, 20, 10, 140]);
            words[13] = 1.0f32.to_bits();
            ObjectDef {
                kind,
                guid: i as u32 + 1,
                name_hash,
                flags: 0,
                resource: None,
                parent: None,
                data: ObjectData { words },
                string: (kind == ObjectKind::String).then(StringDef::default),
                multi: None,
                scripts: Vec::new(),
                responses: Vec::new(),
            }
        })
        .collect();
    let package = Package {
        name: "CustomHUD.fng".into(),
        file_name: String::new(),
        version: 0x20000,
        resources: Vec::new(),
        objects,
        button_count: 0,
        responses: Vec::new(),
        targets: Vec::new(),
    };
    let mut rt = Runtime::new();
    let package = rt.load(package);
    (rt, package)
}

fn texture(rt: &Runtime, package: PackageId, name: u32) -> u32 {
    match rt.tree(package).nodes.iter().find(|node| node.name_hash == name).unwrap().kind {
        NodeKind::Image { texture, .. } => texture,
        _ => panic!("expected gauge image"),
    }
}

fn color(rt: &Runtime, package: PackageId, name: u32) -> [u8; 4] {
    rt.tree(package).nodes.iter().find(|node| node.name_hash == name).unwrap().colour
}

#[test]
fn selected_style_replaces_all_five_installed_preview_images_and_uses_the_engine_scale() {
    let (mut rt, package) = setup();
    let gauges = GaugeSelection { skin: 4, ..Default::default() };
    apply(&mut rt, package, &gauges, 8250.0, true, |_| true);
    assert_eq!(texture(&rt, package, FACE), fe_hash_upper("9000_LINES_04"));
    assert_eq!(texture(&rt, package, FILL), fe_hash_upper("TACH_FILL_04"));
    assert_eq!(texture(&rt, package, NEEDLE), fe_hash_upper("TACH_NEEDLE_04"));
    assert_eq!(texture(&rt, package, TURBO_FACE), fe_hash_upper("TURBO_LINES_04"));
    assert_eq!(texture(&rt, package, TURBO_NEEDLE), fe_hash_upper("TURBO_NEEDLE_04"));
    let turbo = rt.find(package, TURBO_GROUP).unwrap();
    assert!(rt.tree(package).nodes.iter().find(|node| node.index == turbo.index).unwrap().visible);
    apply(&mut rt, package, &gauges, 8250.0, false, |_| true);
    assert!(!rt.tree(package).nodes.iter().find(|node| node.index == turbo.index).unwrap().visible);
}

#[test]
fn backing_needle_and_digit_colors_preserve_alpha_and_restore_the_authored_rgb() {
    let (mut rt, package) = setup();
    let gauges = GaugeSelection {
        skin: 0,
        backing: Some([50, 70, 90]),
        needle: Some([100, 120, 140]),
        digits: Some([150, 170, 190]),
        ..Default::default()
    };
    apply(&mut rt, package, &gauges, 7000.0, true, |_| true);
    assert_eq!(color(&rt, package, FACE), [50, 70, 90, 140]);
    assert_eq!(color(&rt, package, NEEDLE), [100, 120, 140, 140]);
    for &name in DIGITS {
        assert_eq!(color(&rt, package, name), [150, 170, 190, 140]);
    }
    let face = rt.find(package, FACE).unwrap();
    rt.set_alpha(face, 85);
    apply(&mut rt, package, &GaugeSelection::default(), 7000.0, true, |_| true);
    assert_eq!(color(&rt, package, FACE), [10, 20, 30, 85]);
    assert_eq!(color(&rt, package, NEEDLE), [10, 20, 30, 140]);
}

#[test]
fn a_missing_replacement_uses_the_installed_stock_style_and_absent_objects_are_harmless() {
    let (mut rt, package) = setup();
    let gauges = GaugeSelection { skin: 7, ..Default::default() };
    let stock = [
        skin::tach_face_texture(6500.0, 0),
        skin::fill_texture(0),
        skin::needle_texture(0),
        fe_hash_upper("TURBO_LINES_00"),
        fe_hash_upper("TURBO_NEEDLE_00"),
    ];
    apply(&mut rt, package, &gauges, 6500.0, true, |hash| stock.contains(&hash));
    assert_eq!(texture(&rt, package, FACE), stock[0]);
    rt.unload(package);
    apply(&mut rt, package, &gauges, 6500.0, true, |_| true);
    assert!(rt.alpha(ObjectRef { package, index: 0 }).is_none());
}

#[test]
#[ignore = "requires installed garage packages and gauge textures; set NFSMW_GAME_DIR"]
fn installed_custom_hud_previews_bind_selected_styles_and_colors_in_both_rooms() {
    let dir = game_install::GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
    let catalog = crate::ui::Catalog::load(&dir, &crate::ui::SCREEN_FILES);
    let assets = crate::ui::UiAssets::load(&dir).unwrap();
    for name in ["CustomHUD.fng", "CustomHUDColor.fng", "CustomHUD_BACKROOM.fng", "CustomHUDColor_BACKROOM.fng"] {
        let mut rt = crate::ui::runtime();
        let id = rt.load(catalog.find(name).expect("installed garage gauge package").clone());
        show_sample(&mut rt, id);
        for skin in 0..=10 {
            let gauges = GaugeSelection {
                skin,
                backing: Some([30, 100, 200]),
                needle: Some([210, 100, 30]),
                digits: Some([70, 190, 110]),
                ..Default::default()
            };
            rt.update(2.0);
            apply(&mut rt, id, &gauges, 8250.0, true, |hash| assets.texture(hash).is_some());
            for (object, expected) in [
                (FACE, skin::tach_face_texture(8250.0, skin)),
                (FILL, skin::fill_texture(skin)),
                (NEEDLE, skin::needle_texture(skin)),
                (TURBO_FACE, fe_hash_upper(&format!("TURBO_LINES_{skin:02}"))),
                (TURBO_NEEDLE, fe_hash_upper(&format!("TURBO_NEEDLE_{skin:02}"))),
            ] {
                assert_eq!(texture(&rt, id, object), expected, "{name} style {skin}");
                let image = assets.texture(expected).expect("installed gauge image");
                assert!(blackbox_tpk::decode_rgba8(&image).is_some());
                let tree = rt.tree(id);
                let node = tree.nodes.iter().find(|node| node.name_hash == object).unwrap();
                assert!(
                    node.visible && node.world_colour[3] > 0,
                    "{name} style {skin} sample image {object:08x} is visible"
                );
            }
            assert_eq!(&color(&rt, id, FACE)[..3], &[30, 100, 200]);
            assert_eq!(&color(&rt, id, NEEDLE)[..3], &[210, 100, 30]);
            assert_eq!(&color(&rt, id, UNITS)[..3], &[70, 190, 110]);
        }
    }
}
