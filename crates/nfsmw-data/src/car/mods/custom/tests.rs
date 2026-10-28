use super::*;

fn car() -> Customization {
    Customization {
        paints: vec![
            Paint { name: "Red".into(), color: [173, 43, 44], finish: Finish::Gloss },
            Paint { name: "Silver".into(), color: [200, 204, 197], finish: Finish::Metallic },
        ],
        default_paint: 1,
        rim_paints: vec![RimPaint { name: "Black".into(), color: [10, 10, 10] }],
        tints: vec![Tint { name: "Dark".into(), color: [20, 20, 20], opacity: 0.7 }],
        rims: vec![Part { name: "Five spoke".into(), model: "TESTCAR_RIM1".into() }],
        spoilers: Vec::new(),
        roof_scoops: Vec::new(),
        vinyls: Vec::new(),
        vinyl_colors: Vec::new(),
        liveries: vec![Livery { name: "Livery 1".into(), texture: "TESTCAR_LIV1".into() }],
        default_livery: None,
        plate: None,
    }
}

#[test]
fn a_choice_hash_depends_on_the_car_the_kind_and_the_name_not_on_the_case() {
    let red = part_hash("TESTCAR", Kind::Paint, "Red");
    assert_eq!(red, part_hash("testcar", Kind::Paint, "RED"));
    assert_ne!(red, part_hash("TESTCAR", Kind::RimPaint, "Red"));
    assert_ne!(red, part_hash("OTHERCAR", Kind::Paint, "Red"));
}

#[test]
fn pick_finds_the_entry_a_hash_stands_for() {
    let car = car();
    let hash = part_hash("TESTCAR", Kind::Paint, "Silver");
    let found = pick("TESTCAR", Kind::Paint, &car.paints, |p| &p.name, hash).unwrap();
    assert_eq!(found.color, [200, 204, 197]);
    assert!(pick("TESTCAR", Kind::Paint, &car.paints, |p| &p.name, 12345).is_none());
}

#[test]
fn the_registry_answers_by_folder_without_regard_to_case_and_prices_its_choices_at_tier_one() {
    publish("CUSTOMTESTCAR", car());
    assert_eq!(of("customtestcar").unwrap().default_paint, 1);
    assert!(of("NOSUCHCAR").is_none());
    assert_eq!(price_level(part_hash("CUSTOMTESTCAR", Kind::Rim, "Five spoke")), Some(1));
    assert_eq!(price_level(part_hash("CUSTOMTESTCAR", Kind::Rim, "Missing")), None);
}

#[test]
fn a_second_publication_of_a_folder_keeps_the_first() {
    publish("CUSTOMTWICE", car());
    let mut other = car();
    other.default_paint = 0;
    publish("CUSTOMTWICE", other);
    assert_eq!(of("CUSTOMTWICE").unwrap().default_paint, 1);
}

#[test]
fn a_car_without_vinyl_colours_gets_the_built_in_palette() {
    let car = car();
    let palette = vinyl_palette(&car);
    assert_eq!(palette.len(), DEFAULT_VINYL_COLORS.len());
    let mut own = car;
    own.vinyl_colors = vec![VinylColor { name: "Mint".into(), color: [1, 2, 3] }];
    assert_eq!(vinyl_palette(&own), vec![("Mint".to_owned(), [1, 2, 3])]);
}

#[test]
fn a_livery_and_a_vinyl_of_one_name_are_told_apart_by_their_kind() {
    let livery = part_hash("TESTCAR", Kind::Livery, "Livery 1");
    assert_ne!(livery, part_hash("TESTCAR", Kind::Vinyl, "Livery 1"));
    let car = car();
    assert!(pick("TESTCAR", Kind::Livery, &car.liveries, |l| &l.name, livery).is_some());
    assert!(pick("TESTCAR", Kind::Vinyl, &car.vinyls, |v| &v.name, livery).is_none());
}

#[test]
fn a_livery_is_priced_at_the_first_tier_like_the_other_choices() {
    publish("CUSTOMLIVERYCAR", car());
    assert_eq!(price_level(part_hash("CUSTOMLIVERYCAR", Kind::Livery, "Livery 1")), Some(1));
    assert_eq!(price_level(part_hash("CUSTOMLIVERYCAR", Kind::Livery, "Missing")), None);
}

#[test]
fn finishes_name_the_shared_light_materials() {
    assert_eq!(Finish::Gloss.light_material(), "REGPAINTWHITE");
    assert_eq!(Finish::Metallic.light_material(), "METPAINTSILVER");
    assert_eq!(Finish::Chrome.family(), "CHROME");
}

#[test]
fn the_default_paint_is_the_stock_paint_and_the_other_paints_are_the_choices() {
    use crate::car::customization::{Catalog, Category};
    let mut catalog = Catalog::default();
    catalog::paints("TESTCAR", &car(), &mut catalog);
    let silver = part_hash("TESTCAR", Kind::Paint, "Silver");
    let red = part_hash("TESTCAR", Kind::Paint, "Red");
    assert_eq!(catalog.stock[Category::Paint as usize], Some(silver));
    let choices: Vec<u32> = catalog.choices[Category::Paint as usize].iter().map(|c| c.hash).collect();
    assert_eq!(choices, vec![red]);
    assert_eq!(catalog.colors[&silver], [200, 204, 197], "the stock paint still has its colour");
    assert_eq!(catalog.finishes, vec![("Gloss".to_owned(), red)]);
    assert_eq!(catalog.paint_families[&silver], "METAL");
}

#[test]
#[ignore = "requires an installed game"]
fn the_garage_catalogue_of_a_car_with_a_table_has_only_its_own_choices() {
    use crate::car::customization::{Category, Inventory, Selection};
    let dir = game_install::GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
    let mut table = car();
    table.rims.clear(); // the car has no GEOMETRY.BIN in this test
    publish("CATALOGTEST", table);
    let catalog = Inventory::load(&dir).unwrap().catalog(&dir, "CATALOGTEST").unwrap();
    assert_eq!(catalog.choices[Category::Paint as usize].len(), 1);
    assert!(
        catalog.choices[Category::Vinyl as usize].is_empty() && catalog.choices[Category::Spoiler as usize].is_empty()
    );
    assert!(catalog.custom_rim_paint && !catalog.gauge_skins.is_empty());
    let mut selection = Selection::default();
    selection.parts[Category::Paint as usize] = Some(part_hash("CATALOGTEST", Kind::Paint, "Red"));
    catalog.validate(&selection).unwrap();
    selection.parts[Category::Paint as usize] = Some(part_hash("CATALOGTEST", Kind::Paint, "Missing"));
    assert!(catalog.validate(&selection).is_err());
    selection.parts[Category::Paint as usize] = None;
    selection.rim_color = Some([1, 2, 3]);
    catalog.validate(&selection).unwrap();
}

#[test]
fn a_car_that_wears_a_livery_can_take_it_off_and_the_choice_is_priced_like_the_others() {
    let mut dressed = car();
    dressed.default_livery = Some(0);
    let none = no_livery_hash("DRESSEDCAR");
    assert_ne!(none, part_hash("DRESSEDCAR", Kind::Livery, "Livery 1"));
    assert!(hashes("DRESSEDCAR", &dressed).contains(&none));
    assert!(!hashes("DRESSEDCAR", &car()).contains(&none), "a car with no default livery has no such choice");
}
