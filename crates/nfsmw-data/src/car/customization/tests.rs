use super::*;

#[test]
#[ignore = "requires an installed game"]
fn installed_custom_parts_have_materials_and_replace_stock_wheels() {
    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
    let inventory = Inventory::load(&dir).unwrap();
    let options = super::super::LoadOptions { lod: 'A', all_parts: false, preset: None };
    // Inspect all installed rim styles in one geometry/texture pass.
    let wheels: HashMap<_, _> =
        super::super::read_solids(&dir, "WHEELS").unwrap().into_iter().map(|s| (s.name_hash, s)).collect();
    let catalog = inventory.catalog(&dir, "COBALTSS").unwrap();
    let wanted = catalog.choices[Category::Wheels as usize]
        .iter()
        .flat_map(|p| {
            let part = inventory.tables.parts.by_name_hash(p.hash).unwrap();
            let hash = inventory.tables.parts.model_hash(part, 0).unwrap();
            let solid = &wheels[&hash];
            solid.groups.iter().filter_map(move |g| g.diffuse_texture(solid))
        })
        .collect();
    let atlases = super::super::textures::load_with_shared(
        &dir,
        "COBALTSS",
        &wanted,
        &std::collections::HashSet::from(["WHEELS".into()]),
    )
    .unwrap();
    for hash in wanted {
        assert!(atlases.contains_key(&hash), "missing rim atlas {hash}");
    }
    for folder in ["COBALTSS", "BMWM3GTR", "COPMIDSIZE"] {
        let catalog = inventory.catalog(&dir, folder).unwrap();
        let car = inventory.tables.car_type_for_folder(folder).unwrap();
        let stock = stock_parts(&inventory.tables, car);
        let stock_wheel = stock[slot::FRONT_WHEEL].and_then(|p| inventory.tables.parts.model_hash(&p, 0)).unwrap();
        for carbon in [false, true] {
            let mut selection = Selection::default();
            for category in [Category::Spoiler, Category::Roof, Category::Wheels] {
                let choice = catalog.choices[category as usize]
                    .iter()
                    .find(|p| category == Category::Wheels || p.carbon == carbon);
                selection.parts[category as usize] = choice.map(|p| p.hash);
            }
            let mut model = super::super::load_customized(&dir, folder, &options, &selection).unwrap();
            assert!(!model.solids.contains_key(&stock_wheel), "stock wheel must not remain in {folder}");
            let wheels: Vec<_> = model.placements.iter().filter(|p| p.corner.is_some() && !p.is_brake()).collect();
            assert_eq!(wheels.len(), 4);
            assert!(wheels.iter().all(|p| p.solid != stock_wheel));
            for p in model
                .placements
                .iter()
                .filter(|p| [slot::SPOILER, slot::ROOF, slot::FRONT_WHEEL, slot::REAR_WHEEL].contains(&p.slot))
            {
                let solid = &model.solids[&p.solid];
                for group in &solid.groups {
                    if let Some(hash) = group.diffuse_texture(solid) {
                        assert!(
                            model.textures.contains_key(&model.swaps.resolve(hash)),
                            "{} missing diffuse {hash}",
                            solid.name
                        );
                    }
                }
                if carbon && [slot::SPOILER, slot::ROOF].contains(&p.slot) {
                    // Carbon roof scoops can retain a painted base/mount group.
                    let carbon_material = solid
                        .light_material_hashes
                        .iter()
                        .any(|h| model.light_materials.get(h).is_some_and(|m| m.name.contains("CARBON")));
                    let carbon_texture = solid.texture_hashes.iter().any(|h| {
                        model.textures.get(h).is_some_and(|t| t.name.starts_with("CF") || t.name.contains("CARBON"))
                    });
                    assert!(carbon_material || carbon_texture, "{} lost its carbon finish", solid.name);
                }
            }
            if model.paintable {
                model.set_body_color([210, 30, 60]).unwrap();
                for placeholder in ["SPOILER_SKIN1", "ROOF_SKIN"] {
                    assert!(
                        model.textures[&bstring_hash(placeholder)]
                            .data
                            .as_chunks::<4>()
                            .0
                            .iter()
                            .all(|p| *p == [60, 30, 210, 255])
                    );
                }
            }
        }
    }
    println!(
        "all installed rim atlases bound; three cars replace stock wheels; painted and carbon accessories validated"
    );
}

#[test]
fn rim_names_include_manufacturer_style_and_diameter() {
    assert_eq!(rim_label("BBS_STYLE01_18_25_A"), "BBS · Style 1 · 18 in");
    assert_eq!(rim_label("ENKEI_STYLE07_20_25_A"), "ENKEI · Style 7 · 20 in");
    assert_eq!(rim_label("CUSTOM_WHEEL_A"), "CUSTOM WHEEL");
}

#[test]
fn factory_spoilers_and_wheels_do_not_unlock_aerodynamics() {
    let catalog =
        Catalog { stock: [Some(10), Some(20), None, None, Some(30), None, None, None, None], ..Default::default() };
    let mut selection = Selection::default();
    assert!(!catalog.aero_changed(&selection));
    selection.parts[Category::Spoiler as usize] = Some(20);
    assert!(!catalog.aero_changed(&selection), "compare identity rather than presence");
    selection.parts[Category::Wheels as usize] = Some(40);
    assert!(!catalog.aero_changed(&selection));
    selection.parts[Category::Body as usize] = Some(11);
    assert!(catalog.aero_changed(&selection));
    selection.parts[Category::Body as usize] = None;
    selection.parts[Category::Spoiler as usize] = Some(21);
    assert!(catalog.aero_changed(&selection));
}

#[test]
fn forged_slots_and_incomplete_wheel_assemblies_are_rejected() {
    let mut catalog = Catalog::default();
    catalog.choices[Category::Body as usize].push(Choice {
        hash: 42,
        label: "Kit".into(),
        language_hash: None,
        carbon: false,
    });
    let mut selection = Selection::default();
    selection.parts[Category::Spoiler as usize] = Some(42);
    assert!(catalog.validate(&selection).is_err());
    selection.parts[Category::Spoiler as usize] = None;
    selection.parts[Category::Body as usize] = Some(42);
    assert!(catalog.validate(&selection).is_ok());
    assert!(validate_placements(&selection, &[]).is_err());
    selection = Selection::default();
    selection.parts[Category::Wheels as usize] = Some(123);
    let placements: Vec<_> = (0..4)
        .map(|i| Placement {
            solid: 123,
            transform: glam::Mat4::IDENTITY,
            left_brake: false,
            slot: match i < 2 {
                true => slot::FRONT_WHEEL,
                false => slot::REAR_WHEEL,
            },
            corner: Some(i),
        })
        .collect();
    assert!(validate_placements(&selection, &placements[..3]).is_err());
    assert!(validate_placements(&selection, &placements).is_ok());
}

#[test]
#[ignore = "requires an installed game"]
fn installed_customization_catalogs_and_model_assemblies() {
    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
    let inventory = Inventory::load(&dir).unwrap();
    let options = super::super::LoadOptions { lod: 'A', all_parts: false, preset: None };
    let mut cars = 0;
    let mut counts = [0; 5];
    for folder in super::super::list(&dir) {
        let Some(car) = inventory.tables.car_type_for_folder(&folder) else { continue };
        if ![0, 1].contains(&car.usage_type) {
            continue;
        }
        let catalog = inventory.catalog(&dir, &folder).unwrap();
        cars += 1;
        for category in Category::MODELS {
            let choices = &catalog.choices[category as usize];
            counts[category as usize] += choices.len();
            // Every advertised identity must resolve for this car; assembly probes use both ends
            // of each category, covering car-specific and shared/carbon variants without reloading
            // every identical shared pack hundreds of times.
            for choice in choices {
                let mut selection = Selection::default();
                selection.parts[category as usize] = Some(choice.hash);
                let slots = resolve(&inventory.tables, car, &selection).unwrap();
                assert_eq!(slots[category.slot()].unwrap().name_hash, choice.hash);
                if category == Category::Body {
                    let kit = super::super::stock::kit_number(&inventory.tables, &slots[slot::BODY].unwrap());
                    for s in slot::DAMAGE0 {
                        if let Some(part) = slots[s] {
                            let hash = inventory.tables.parts.model_hash(&part, 0).unwrap();
                            let name = format!("{}_KIT{kit:02}_{}_A", car.type_name, LAYOUT.slots[s].name);
                            assert_eq!(hash, bstring_hash(&name));
                        }
                    }
                }
            }
            for choice in choices.first().into_iter().chain(choices.last()).take(match choices.len() {
                1 => 1,
                _ => 2,
            }) {
                let mut selection = Selection::default();
                selection.parts[category as usize] = Some(choice.hash);
                let model = super::super::load_customized(&dir, &folder, &options, &selection).unwrap();
                assert!(model.placements.iter().any(|p| p.slot == category.slot()));
                if category == Category::Wheels {
                    assert_eq!(model.placements.iter().filter(|p| p.corner.is_some() && !p.is_brake()).count(), 4);
                }
            }
        }
    }
    assert!(cars >= 40);
    assert!(counts.iter().all(|count| *count > 20), "must exercise all five categories: {counts:?}");
    println!("{cars} car catalogs, installed choices (body/spoiler/hood/roof/rims): {counts:?}");
}

#[test]
#[ignore = "requires an installed game"]
fn installed_appearance_choices_resolve_paint_and_tint_materials() {
    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
    let inventory = Inventory::load(&dir).unwrap();
    let options = super::super::LoadOptions { lod: 'A', all_parts: false, preset: None };
    for folder in ["COBALTSS", "BMWM3GTR"] {
        let catalog = inventory.catalog(&dir, folder).unwrap();
        for category in [Category::Paint, Category::WindowTint] {
            let choices = &catalog.choices[category as usize];
            assert!(!choices.is_empty(), "{folder}: no {} choices", category.label());
            for choice in choices.first().into_iter().chain(choices.last()) {
                let mut selection = Selection::default();
                selection.parts[category as usize] = Some(choice.hash);
                catalog.validate(&selection).unwrap();
                let mut model = super::super::load_customized(&dir, folder, &options, &selection).unwrap();
                let part = inventory.tables.parts.by_name_hash(choice.hash).unwrap();
                let material = inventory.tables.parts.attribute(part, "LIGHT_MATERIAL_NAME").unwrap();
                assert!(model.light_materials.contains_key(&material));
                if category == Category::WindowTint {
                    assert_eq!(model.window_tint_material, Some(material));
                    continue;
                }
                let expected = super::super::Paint::of(&inventory.tables, part);
                assert_eq!(model.paint.as_ref(), Some(&expected));
                model.set_body_color([10, 80, 230]).unwrap();
                assert_eq!(model.paint.as_ref().unwrap().light_material, Some(material));
                assert_eq!(model.paint.as_ref().unwrap().rgb, [10, 80, 230]);
            }
            println!("{folder}: {} {} choices", choices.len(), category.label());
        }
        let mut invalid = Selection::default();
        invalid.parts[Category::Paint as usize] =
            catalog.choices[Category::WindowTint as usize].first().map(|p| p.hash);
        assert!(catalog.validate(&invalid).is_err());
        let stock = super::super::load_customized(&dir, folder, &options, &Selection::default()).unwrap();
        let car = inventory.tables.car_type_for_folder(folder).unwrap();
        let slots = stock_parts(&inventory.tables, car);
        assert_eq!(stock.paint, slots[slot::BASE_PAINT].map(|p| super::super::Paint::of(&inventory.tables, &p)));
    }
}

#[test]
#[ignore = "requires an installed game"]
fn installed_fixed_vinyls_recolor_without_losing_artwork_and_rim_paint_is_isolated() {
    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
    let inventory = Inventory::load(&dir).unwrap();
    let catalog = inventory.catalog(&dir, "COBALTSS").unwrap();
    let options = super::super::LoadOptions { lod: 'A', all_parts: false, preset: None };
    assert_eq!(catalog.finishes.len(), 5);
    assert_eq!(catalog.gauge_skins, (0..=10).collect::<Vec<_>>());
    let mut selection = Selection::default();
    selection.parts[Category::Wheels as usize] = catalog.paintable_wheels.iter().copied().min();
    let before = super::super::load_customized_driving(&dir, "COBALTSS", &options, &selection).unwrap();
    selection.parts[Category::RimPaint as usize] = Some(catalog.choices[Category::RimPaint as usize][0].hash);
    selection.rim_color = Some([25, 180, 230]);
    catalog.validate(&selection).unwrap();
    let painted = super::super::load_customized_driving(&dir, "COBALTSS", &options, &selection).unwrap();
    let rim = bstring_hash("__GARAGE_RIM_PAINT");
    assert!(painted.light_materials.contains_key(&rim));
    let eligible: std::collections::HashSet<_> = before
        .placements
        .iter()
        .chain(before.driving_lods.levels.iter().flat_map(|l| &l.placements))
        .filter(|p| [slot::FRONT_WHEEL, slot::REAR_WHEEL].contains(&p.slot))
        .map(|p| p.solid)
        .collect();
    let mut replaced = 0;
    for (hash, solid) in &before.solids {
        let after = &painted.solids[hash];
        for (old, new) in solid.light_material_hashes.iter().zip(&after.light_material_hashes) {
            let change =
                eligible.contains(hash) && ["MAGSILVER", "MAGCHROME", "MAGGUNMETAL"].map(bstring_hash).contains(old);
            assert_eq!(
                *new,
                match change {
                    true => rim,
                    false => *old,
                }
            );
            replaced += usize::from(change);
        }
    }
    assert!(replaced > 0, "test wheel must contain paintable rim materials");
    let vinyls = &catalog.choices[Category::Vinyl as usize];
    assert!(vinyls.len() > 50);
    for remap in 0..=2 {
        let choice = vinyls.iter().find(|p| catalog.vinyl_remaps[&p.hash] == remap).unwrap();
        selection.parts[Category::Vinyl as usize] = Some(choice.hash);
        selection.vinyl_colors = [Some([240, 30, 10]), Some([30, 230, 60]), None];
        catalog.validate(&selection).unwrap();
        let mut model = super::super::load_customized_driving(&dir, "COBALTSS", &options, &selection).unwrap();
        let layer_hash = bstring_hash("__GARAGE_VINYL");
        let layer = model.textures[&layer_hash].clone();
        assert!(layer.data.as_chunks::<4>().0.iter().any(|p| p[3] > 0));
        model.set_body_color([10, 80, 200]).unwrap();
        assert_eq!(model.textures[&layer_hash].data, layer.data, "recolor must retain artwork source");
        let skins: Vec<_> =
            model.textures.values().filter(|t| t.name.ends_with("_SKIN1") && t.name != "SPOILER_SKIN1").collect();
        assert!(!skins.is_empty());
        for skin in skins {
            assert_eq!((skin.width, skin.height), (layer.width, layer.height));
            for (art, pixel) in layer.data.as_chunks::<4>().0.iter().zip(skin.data.as_chunks::<4>().0) {
                let a = u32::from(art[3]);
                for (i, base) in [200u32, 80, 10].into_iter().enumerate() {
                    assert_eq!(u32::from(pixel[i]), (u32::from(art[i]) * a + base * (255 - a) + 127) / 255);
                }
                assert_eq!(pixel[3], 255);
            }
        }
        println!("{} fixed vinyls, {} recolor channels checked", vinyls.len(), remap);
    }
    selection.parts[Category::Wheels as usize] = None;
    assert!(catalog.validate(&selection).is_err(), "stock wheels cannot use rim paint");
}
