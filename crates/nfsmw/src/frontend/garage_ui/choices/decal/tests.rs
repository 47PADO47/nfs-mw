use super::*;
use crate::garage::{Car, Selection};
use nfsmw_data::car::customization::{Catalog, Category};

fn fixture() -> Model {
    let mut catalog = Catalog::default();
    catalog.stock[Category::Body as usize] = Some(42);
    for (zone, data) in catalog.decals.iter_mut().enumerate() {
        data.body_positions.insert(
            42,
            match zone {
                2 | 3 => 6,
                _ => 1,
            },
        );
    }
    Model {
        cars: vec![Car { folder: "CAR".into(), parts: Some(catalog.into()), ..Default::default() }],
        draft: Selection { car: "CAR".into(), ..Default::default() },
        ..Default::default()
    }
}

#[test]
fn decal_locations_and_door_positions_use_native_icons_and_page_routes_in_both_rooms() {
    let model = fixture();
    for backroom in [false, true] {
        let locations = zones(&model, backroom);
        assert_eq!(locations.iter().map(|item| item.texture).collect::<Vec<_>>(), ZONE_ICONS);
        for (zone, item) in locations.iter().enumerate() {
            assert!(item.enabled);
            let expected = match zone {
                2 | 3 => Page::DecalSlots { zone, backroom },
                _ => Page::Choices { row: Row::Decal(zone, 0), backroom },
            };
            assert_eq!(item.next, Some(expected));
        }
        for (zone, expected) in [(2, LEFT_SLOTS), (3, RIGHT_SLOTS)] {
            let positions = slots(&model, zone, backroom);
            assert_eq!(positions.iter().map(|item| item.texture).collect::<Vec<_>>(), expected);
            for (position, item) in positions.iter().enumerate() {
                assert_eq!(item.next, Some(Page::Choices { row: Row::Decal(zone, position), backroom }));
            }
        }
    }
    for option in [0, 1, 100] {
        assert_eq!(super::super::texture(Row::Decal(0, 0), &model, option), CHOICE_ICON);
    }
}

#[test]
fn custom_non_door_positions_keep_a_selector_without_invented_position_icons() {
    let mut model = fixture();
    model.cars[0].parts.as_mut().unwrap().decals[0].body_positions.insert(42, 3);
    assert_eq!(zones(&model, false)[0].next, Some(Page::DecalSlots { zone: 0, backroom: false }));
    assert_eq!(slots(&model, 0, false).iter().map(|item| item.texture).collect::<Vec<_>>(), vec![ZONE_ICONS[0]; 3]);
}

#[test]
#[ignore = "requires installed original garage UI textures"]
fn installed_decal_navigation_icons_exist_and_non_door_zones_open_choices_directly() {
    let dir = game_install::GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
    let assets = crate::ui::UiAssets::load(&dir).unwrap();
    let model = fixture();
    for backroom in [false, true] {
        let locations = super::super::items(Page::DecalZones { backroom }, &model);
        for item in &locations {
            assert!(assets.texture(item.texture).is_some(), "missing decal zone bitmap {:08x}", item.texture);
        }
        for zone in [2, 3] {
            let options = super::super::items(Page::DecalSlots { zone, backroom }, &model);
            assert_eq!(options.len(), 6);
            for item in options {
                assert!(assets.texture(item.texture).is_some(), "missing decal position bitmap {:08x}", item.texture);
            }
        }
        for zone in [0, 1, 4, 5] {
            assert_eq!(locations[zone].next, Some(Page::Choices { row: Row::Decal(zone, 0), backroom }));
        }
    }
    assert!(assets.texture(CHOICE_ICON).is_some());
}
