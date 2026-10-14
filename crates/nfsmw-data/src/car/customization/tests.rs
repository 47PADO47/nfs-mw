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
    let catalog = Catalog { stock: [Some(10), Some(20), None, None, Some(30)], ..Default::default() };
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
        for category in Category::ALL {
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
