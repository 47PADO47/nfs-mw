use super::*;

#[test]
#[ignore = "requires an installed game"]
fn installed_garage_header_keeps_committed_cart_separate_from_highlight() {
    let dir = game_install::GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
    let pricing = crate::garage::pricing::Pricing::load(&dir).unwrap();
    let parts = nfsmw_data::car::customization::Catalog::load(&dir, "COBALTSS").unwrap();
    let bodies = &parts.choices[Part::Body as usize];
    assert!(bodies.len() >= 3);
    let baseline = Selection { car: "COBALTSS".into(), ..Default::default() };
    let mut cart = baseline.clone();
    cart.parts.parts[Part::Body as usize] = Some(bodies[0].hash);
    let mut draft = cart.clone();
    draft.parts.parts[Part::Body as usize] = Some(bodies[bodies.len() - 1].hash);
    let model = Model {
        shop: true,
        shop_cash: Some(20_000),
        pricing: Some(pricing),
        draft,
        cars: vec![crate::garage::Car { folder: "COBALTSS".into(), parts: Some(parts.into()), ..Default::default() }],
        ..Default::default()
    };
    let page = Page::Choices { row: Row::Part(Part::Body), backroom: false };
    let header = frame(page, &model, &baseline, &cart, None);
    let pricing = model.pricing.as_ref().unwrap();
    let expected =
        pricing.item(Row::Part(Part::Body), &model.draft, &baseline, model.selected().unwrap(), false).unwrap();
    let committed = pricing.items(model.selected().unwrap(), &baseline, &cart, false, true).unwrap();
    assert_eq!(header.selected, Some((expected.cost, expected.trade_in)));
    assert_eq!(header.totals, Some(Totals::of(&committed)));
    let catalog = crate::ui::Catalog::load(&dir, &crate::ui::SCREEN_FILES);
    let mut rt = crate::ui::runtime();
    let package = rt.load(catalog.find(page.package()).unwrap().clone());
    apply(&mut rt, package, &header);
    let string = |rt: &Runtime, hash| rt.string_info(rt.find(package, hash).unwrap()).unwrap().text;
    assert_eq!(string(&rt, 0x6dfa_e796), crate::i18n::fmt::grouped(i64::from(expected.cost)));
    let total = string(&rt, 0x3614_0053);
    let mut preview = model.clone();
    preview.draft.parts.parts[Part::Body as usize] = None;
    apply(&mut rt, package, &frame(page, &preview, &baseline, &cart, None));
    assert_eq!(string(&rt, 0x3614_0053), total, "changing the highlight cannot change the committed cart total");
    assert_eq!(string(&rt, 0x6dfa_e796), "0", "restoring the stock model costs nothing");
    let cart_page = Page::Cart { backroom: false, quick: false };
    let unchecked = frame(cart_page, &preview, &baseline, &cart, None);
    assert_eq!(unchecked.totals, Some(Totals::default()), "unchecked cart items must not contribute to checkout");

    let mut backroom = model.clone();
    backroom.backroom = true;
    backroom.shop_markers = Some(100);
    backroom.shop_marker_counts[8] = 2;
    backroom.shop_marker_counts[9] = 3;
    backroom.shop_marker_counts[1] = 50;
    let page = Page::Cart { backroom: true, quick: false };
    let header = frame(page, &backroom, &baseline, &cart, Some(Section::Parts));
    assert_eq!(header.markers, 5, "Parts cart excludes performance and visual markers");
    assert_eq!(header.marker_rows.len(), 5);
    assert_eq!(header.marker_rows[0].0, 1);
    assert_eq!(header.marker_rows[1].0, 3);
    let package = rt.load(catalog.find(page.package()).unwrap().clone());
    apply(&mut rt, package, &header);
    let texts = |rt: &Runtime, hash| {
        strings(rt, rt.find(package, hash).unwrap())
            .into_iter()
            .map(|object| rt.string_info(object).unwrap().text)
            .collect::<Vec<_>>()
    };
    let counts = texts(&rt, blackbox_feng::fe_hash_upper("MARKER_NUM_1"));
    assert!(!counts.is_empty());
    assert!(counts.iter().all(|text| text == "1"));
    assert!(texts(&rt, 0x1866_1565).iter().all(|text| text == "4"));
    backroom.draft = baseline.clone();
    let unchecked = frame(page, &backroom, &baseline, &cart, Some(Section::Parts));
    assert_eq!(unchecked.marker_rows[0].0, 2);
    assert_eq!(unchecked.totals.unwrap().markers, 0);
}
