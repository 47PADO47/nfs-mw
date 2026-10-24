use super::*;

#[test]
fn gross_cost_and_trade_in_do_not_collapse_into_net_refund() {
    let items = [
        Item { entity: 11, level: 1, cost: 100, trade_in: 200, marker: None },
        Item { entity: 14, level: 1, cost: 50, trade_in: 0, marker: None },
    ];
    assert_eq!(Totals::of(&items), Totals { cost: 150, trade_in: 200, net: -50, markers: 0 });
}

#[test]
#[ignore = "requires an installed game"]
fn installed_garage_pricing_keeps_gross_trade_and_color_only_parent_charge() {
    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
    let physics = nfsmw_data::car::physics::PhysicsData::load(&dir).unwrap();
    let tables = CarTables::load(&dir).unwrap();
    let car_type = tables.car_type_for_folder("COBALTSS").unwrap();
    let car = Car {
        folder: "COBALTSS".into(),
        performance: nfsmw_data::car::physics::PerformanceCatalog::read(physics.database(), &car_type.type_name)
            .unwrap(),
        parts: Some(nfsmw_data::car::customization::Catalog::load(&dir, "COBALTSS").unwrap().into()),
        ..Default::default()
    };
    let car = &car;
    let pricing = Pricing::load(&dir).unwrap();
    let catalog = car.parts.as_ref().unwrap();
    let body = &catalog.choices[Category::Body as usize];
    let mut installed = Selection { car: car.folder.clone(), ..Default::default() };
    installed.parts.parts[Category::Body as usize] = Some(body.last().unwrap().hash);
    let mut selected = installed.clone();
    selected.parts.parts[Category::Body as usize] = Some(body.first().unwrap().hash);
    let body = pricing.item(Row::Part(Category::Body), &selected, &installed, car, false).unwrap();
    let installed_level = pricing.level(installed.parts.parts[Category::Body as usize]);
    assert_eq!(body.trade_in, pricing.prices.cost(body.entity, installed_level).unwrap() / 4);
    let totals = Totals::of(&[body]);
    assert_eq!(totals.cost, u64::from(body.cost));
    assert_eq!(totals.trade_in, u64::from(body.trade_in));
    assert_eq!(totals.net, i64::from(body.cost) - i64::from(body.trade_in));

    installed.parts.parts[Category::Vinyl as usize] = catalog.choices[Category::Vinyl as usize]
        .iter()
        .find(|part| pricing.level(Some(part.hash)) > 0)
        .map(|part| part.hash);
    assert!(installed.parts.parts[Category::Vinyl as usize].is_some());
    installed.gauges.skin = 4;
    selected = installed.clone();
    selected.parts.vinyl_colors[0] = Some([12, 34, 56]);
    selected.gauges.needle = Some([65, 43, 21]);
    let ordinary = pricing.items(car, &installed, &selected, false, false).unwrap();
    assert_eq!(ordinary.len(), 2);
    assert!(ordinary.iter().any(|item| item.entity == 40 && item.cost > 0));
    assert!(ordinary.iter().any(|item| item.entity == 17 && item.cost > 0));
    let mut settings = nfsmw_career::settings::CareerSettings::new_career();
    settings.flags.insert(nfsmw_career::settings::Flags::CAREER_COMPLETE);
    settings.cash = u32::MAX;
    let quote = nfsmw_career::economy::shop::quote(
        &settings,
        &nfsmw_career::economy::markers::MarkerInventory::empty(),
        &ordinary,
    )
    .unwrap();
    assert_eq!(u64::from(quote.total), Totals::of(&ordinary).cost);
    assert_eq!(quote.refund, 0);
    let backroom = pricing.items(car, &installed, &selected, true, false).unwrap();
    assert_eq!(Totals::of(&backroom).markers, 2);
    assert!(backroom.iter().all(|item| item.cost == 0));

    installed.parts.vinyl_colors[0] = Some([12, 34, 56]);
    installed.parts.vinyl_color_parts[0] = Some(10);
    selected = installed.clone();
    selected.parts.vinyl_color_parts[0] = Some(11);
    let identity_only = pricing.items(car, &installed, &selected, false, false).unwrap();
    assert_eq!(identity_only.len(), 1);
    assert_eq!(identity_only[0].entity, 40);
    assert!(identity_only[0].cost > 0, "an equal-RGB native part change still queues its parent vinyl");
}
