//! Category ordering and installed selections offered by the original garage.

use super::{Page, Section};
use crate::garage::{Model, menu_rows::Row};
use nfsmw_data::car::customization::Category as Part;
use nfsmw_data::car::physics::PerformanceCategory as Hardware;

mod availability;
pub(super) mod decal;

pub(super) fn can_enter_backroom(model: &Model) -> bool {
    model.shop
        && !model.backroom
        && model.selected().is_some_and(|car| !car.hero)
        && model.shop_marker_counts[1..].iter().any(|count| *count > 0)
}

pub(super) struct Item {
    pub label: String,
    pub texture: u32,
    pub enabled: bool,
    pub next: Option<Page>,
    pub option: Option<usize>,
    pub row: Option<Row>,
}

impl Item {
    fn page(label: &str, texture: u32, next: Page, enabled: bool) -> Self {
        Self { label: label.into(), texture, next: Some(next), option: None, row: None, enabled }
    }
}

pub(super) fn items(page: Page, model: &Model) -> Vec<Item> {
    let mut entries = build(page, model);
    if model.shop && (page.backroom() || (page == Page::Main && model.backroom)) {
        entries.retain(|entry| availability::entry(page, entry, model));
    }
    entries
}

fn build(page: Page, model: &Model) -> Vec<Item> {
    match page {
        Page::Main => [
            ("Parts", 0x6e0c_a66c, Section::Parts),
            ("Performance", 0x3987_d054, Section::Performance),
            ("Visual", 0x3e31_ba56, Section::Visual),
        ]
        .into_iter()
        .filter(|(_, _, section)| match section {
            Section::Parts => {
                model.selected().filter(|car| !car.hero).and_then(|car| car.parts.as_ref()).is_some_and(|catalog| {
                    Part::MODELS.into_iter().any(|part| !catalog.choices[part as usize].is_empty())
                })
            }
            Section::Performance => model
                .selected()
                .filter(|car| !car.hero)
                .is_some_and(|car| car.performance.categories.iter().any(|category| category.max_level > 0)),
            Section::Visual => true,
        })
        .map(|(label, texture, section)| {
            Item::page(label, texture, Page::Categories { section, backroom: model.backroom }, true)
        })
        .collect(),
        Page::Categories { section: Section::Parts, backroom } => [
            (Part::Body, 0x028c_24f6),
            (Part::Spoiler, 0xbb03_4ea6),
            (Part::Wheels, 0x0294_d2a3),
            (Part::Hood, 0x028f_7092),
            (Part::Roof, 0x7916_5861),
        ]
        .into_iter()
        .map(|(part, texture)| {
            let row = Row::Part(part);
            let available = model
                .selected()
                .and_then(|car| car.parts.as_ref())
                .is_some_and(|parts| !parts.choices[part as usize].is_empty());
            let next = match part {
                Part::Wheels => Page::RimBrands { backroom },
                Part::Spoiler => Page::Spoilers { group: 0, backroom },
                _ => Page::Choices { row, backroom },
            };
            Item::page(part.label(), texture, next, available)
        })
        .collect(),
        Page::Categories { section: Section::Performance, backroom } => [
            Hardware::Engine,
            Hardware::Transmission,
            Hardware::Chassis,
            Hardware::Nitrous,
            Hardware::Tires,
            Hardware::Brakes,
            Hardware::Induction,
        ]
        .into_iter()
        .map(|part| {
            let row = match backroom {
                true => Row::Junkman(part),
                false => Row::Performance(part),
            };
            let enabled = model.selected().is_some_and(|car| car.performance.categories[part as usize].max_level > 0);
            let mut texture = hardware_texture(part, backroom);
            if part == Hardware::Induction && model.selected().is_some_and(|car| !car.performance_details.turbo) {
                texture = match backroom {
                    true => 0x6300_71e4,
                    false => 0x9360_3dfb,
                };
            }
            Item::page(part.label(), texture, Page::Choices { row, backroom }, enabled)
        })
        .collect(),
        Page::Categories { section: Section::Visual, backroom } => visual(model, backroom),
        Page::DecalZones { backroom } => decal::zones(model, backroom),
        Page::DecalSlots { zone, backroom } => decal::slots(model, zone, backroom),
        Page::Choices { row, .. } => options(row, model),
        Page::RimBrands { backroom } => rim_brands(model, backroom),
        Page::Rims { brand, radius, .. } => rim_options(model, brand, radius),
        Page::Spoilers { group, .. } => group_options(model, Part::Spoiler, group),
        Page::VinylGroups { backroom } => vinyl_groups(model, backroom),
        Page::Vinyls { group, .. } => group_options(model, Part::Vinyl, group),
        _ => Vec::new(),
    }
}

fn visual(model: &Model, backroom: bool) -> Vec<Item> {
    let mut result = vec![
        Item::page(
            "Paint",
            match backroom {
                true => 0x0db8_9e17,
                false => 0xa3b7_6154,
            },
            Page::Paint { color: 0, backroom },
            Row::Color(0).enabled(model),
        ),
        Item::page(
            "Vinyls",
            match backroom {
                true => 0xd35f_04c0,
                false => 0x5577_8e5a,
            },
            Page::VinylGroups { backroom },
            true,
        ),
    ];
    if !backroom {
        if model
            .selected()
            .filter(|car| !car.hero)
            .and_then(|car| car.parts.as_ref())
            .is_some_and(|catalog| !catalog.choices[Part::Wheels as usize].is_empty())
        {
            result.push(Item::page(
                "Rim Paint",
                0xd223_f84a,
                Page::Paint { color: 1, backroom },
                Row::Color(1).enabled(model),
            ));
        }
        result.push(Item::page(
            "Window Tint",
            0x3f23_165c,
            Page::Choices { row: Row::Part(Part::WindowTint), backroom },
            true,
        ));
    }
    result.push(Item::page(
        "Decals",
        match backroom {
            true => 0xa913_5927,
            false => 0xda1d_ae54,
        },
        Page::DecalZones { backroom },
        true,
    ));
    if !backroom {
        result.push(Item::page("Numbers", 0x45a1_c644, Page::Numbers, Row::Number(0).enabled(model)));
    }
    result.push(Item::page(
        "Custom Gauges",
        match backroom {
            true => 0x8ba6_02fc,
            false => 0x028f_88bc,
        },
        Page::Choices { row: Row::Gauge, backroom },
        true,
    ));
    result
}

fn options(row: Row, model: &Model) -> Vec<Item> {
    let (labels, _) = row.options(model);
    let mut result: Vec<_> = labels
        .into_iter()
        .enumerate()
        .filter(|(index, _)| !matches!(row, Row::Performance(_) | Row::Junkman(_)) || *index > 0)
        .map(|(index, label)| Item {
            label,
            texture: texture(row, model, index),
            enabled: row.enabled(model),
            next: None,
            option: Some(index),
            row: Some(row),
        })
        .collect();
    if let Row::Performance(category) = row {
        let unlocked = crate::career::CareerHandle::shared()
            .with_loaded(|career| {
                career.settings.flags.contains(nfsmw_career::settings::Flags::QUICK_RACE_JUNKMAN)
                    || career.career_completed_once()
            })
            .unwrap_or(false);
        if !model.shop
            && unlocked
            && model.selected().is_some_and(|car| !car.hero)
            && Row::Junkman(category).enabled(model)
        {
            result.push(Item {
                label: "Junkman".into(),
                texture: 0xb8c8_c0d4,
                enabled: true,
                next: None,
                option: Some(1),
                row: Some(Row::Junkman(category)),
            });
        }
    }
    result
}

pub(super) fn texture(row: Row, model: &Model, index: usize) -> u32 {
    if let Row::Part(part) = row {
        let carbon = index
            .checked_sub(1)
            .and_then(|index| model.selected()?.parts.as_ref()?.choices[part as usize].get(index))
            .is_some_and(|choice| choice.carbon);
        if carbon {
            match (part, model.backroom) {
                (Part::Hood, false) => return 0x6849_5926,
                (Part::Hood, true) => return 0x2478_e136,
                (Part::Roof, false) => return 0xfc61_8215,
                (Part::Roof, true) => return 0xcd6b_4e26,
                (Part::Spoiler, false) => return 0x4d1c_18ba,
                (Part::Spoiler, true) => return 0x611d_142a,
                _ => {}
            }
        }
    }
    match row {
        Row::Part(Part::Body) => 0x028c_24f6,
        Row::Part(Part::Spoiler) => 0xbb03_4ea6,
        Row::Part(Part::Wheels) => 0x0294_d2a3,
        Row::Part(Part::Hood) => 0x028f_7092,
        Row::Part(Part::Roof) => 0x7916_5861,
        Row::Performance(_) | Row::Junkman(_) => 0xb8c8_c0d4,
        Row::Decal(_, _) => decal::CHOICE_ICON,
        Row::Number(_) => 0x697b_4ad4,
        Row::Part(Part::Vinyl) => 0x5577_8e5a,
        Row::Part(Part::WindowTint) => 0x3f23_165c,
        Row::Gauge => 0x028f_88bc,
        _ => 0x3e31_ba56,
    }
}

const RIM_BRANDS: [(&str, u32, u32); 11] = [
    ("5Zigen", 0x352d_08d1, 0xb0da_3de4),
    ("ADR", 0x9136, 0xf224_a729),
    ("BBS", 0x9536, 0xf224_ab29),
    ("Enkei", 0x02b7_7feb, 0x0e38_de9e),
    ("Konig", 0x0324_ac97, 0x0ea6_0b4a),
    ("Lowenhart", 0x48e2_5793, 0xafc6_b9cb),
    ("Racing Hart", 0xdd54_4a02, 0x27eb_d095),
    ("OZ", 0x648, 0x6c2f_a9db),
    ("Volk", 0x1e6a3b, 0x36c5_3e8e),
    ("Roja", 0x1c386b, 0x36c2_d130),
    ("Stock", 0, 0x0f39_90b6),
];

fn rim_brands(model: &Model, backroom: bool) -> Vec<Item> {
    let mut result = Vec::new();
    let Some(catalog) = model.selected().and_then(|car| car.parts.as_ref()) else { return result };
    let mut brands: Vec<_> = catalog.rim_metadata.values().map(|(brand, _)| *brand).collect();
    brands.sort_unstable();
    brands.dedup();
    result.push(Item::page("Stock", 0x0f39_90b6, Page::Rims { brand: 0, radius: 0, backroom }, true));
    for (label, brand, texture) in RIM_BRANDS {
        if brand == 0 || !brands.contains(&brand) {
            continue;
        }
        let radius = initial_radius(model, brand);
        result.push(Item::page(label, texture, Page::Rims { brand, radius, backroom }, true));
    }
    for brand in
        brands.into_iter().filter(|brand| *brand != 0 && !RIM_BRANDS.iter().any(|(_, known, _)| known == brand))
    {
        let radius = initial_radius(model, brand);
        result.push(Item::page(
            &format!("Rims {brand:08X}"),
            0x0294_d2a3,
            Page::Rims { brand, radius, backroom },
            true,
        ));
    }
    result
}

pub(super) fn radii(model: &Model, brand: u32) -> Vec<u32> {
    let Some(catalog) = model.selected().and_then(|car| car.parts.as_ref()) else { return Vec::new() };
    let mut radii: Vec<_> =
        catalog.rim_metadata.values().filter(|(candidate, _)| *candidate == brand).map(|(_, radius)| *radius).collect();
    radii.sort_unstable();
    radii.dedup();
    radii
}

fn initial_radius(model: &Model, brand: u32) -> u32 {
    let sizes = radii(model, brand);
    let current = model
        .selected()
        .and_then(|car| car.parts.as_ref())
        .and_then(|catalog| catalog.rim_metadata.get(&model.draft.parts.parts[Part::Wheels as usize]?))
        .map(|(_, radius)| *radius)
        .filter(|radius| sizes.contains(radius));
    current.or_else(|| sizes.first().copied()).unwrap_or(0)
}

fn rim_options(model: &Model, brand: u32, radius: u32) -> Vec<Item> {
    let mut options = options(Row::Part(Part::Wheels), model);
    if brand == 0 {
        options.truncate(1);
        return options;
    }
    let Some(catalog) = model.selected().and_then(|car| car.parts.as_ref()) else { return Vec::new() };
    options.retain(|item| {
        item.option
            .and_then(|index| index.checked_sub(1))
            .and_then(|index| catalog.choices[Part::Wheels as usize].get(index))
            .and_then(|part| catalog.rim_metadata.get(&part.hash))
            .is_some_and(|entry| *entry == (brand, radius))
    });
    options
}

fn hardware_texture(part: Hardware, backroom: bool) -> u32 {
    let pair = match part {
        Hardware::Engine => (0xc15c_94e6, 0x4f42_4e0f),
        Hardware::Transmission => (0x01a2_9ffa, 0xd142_d3e3),
        Hardware::Chassis => (0x1784_75e7, 0x0019_0eb6),
        Hardware::Nitrous => (0x9701_bde4, 0x6fea_04c8),
        Hardware::Tires => (0x06e8_e477, 0x7373_f1ef),
        Hardware::Brakes => (0xbaa2_3a28, 0x4887_f351),
        Hardware::Induction => (0x06ef_789c, 0x12fe_30a5),
    };
    match backroom {
        true => pair.1,
        false => pair.0,
    }
}

const VINYLS: [(&str, u32); 8] = [
    ("Flames", 0xf814_8554),
    ("Tribal", 0x192d_84da),
    ("Stripes", 0xf735_2706),
    ("Racing Flags", 0x1223_cc89),
    ("National Flags", 0xbc44_bbcb),
    ("Body", 0x694c_a0ca),
    ("Unique", 0x1b3a_8dd3),
    ("Contest", 0x1ba5_08fc),
];

fn vinyl_groups(model: &Model, backroom: bool) -> Vec<Item> {
    let mut result = vec![Item {
        label: "Stock".into(),
        texture: 0x21f3_d114,
        enabled: true,
        next: None,
        option: Some(0),
        row: None,
    }];
    let Some(catalog) = model.selected().and_then(|car| car.parts.as_ref()) else { return result };
    let mut groups: Vec<u8> = catalog.choices[Part::Vinyl as usize]
        .iter()
        .filter_map(|choice| catalog.part_groups.get(&choice.hash).copied())
        .collect();
    groups.sort_unstable();
    groups.dedup();
    for group in groups {
        let (label, texture) = VINYLS.get(group as usize).copied().unwrap_or(("Custom Vinyls", 0x5577_8e5a));
        result.push(Item::page(label, texture, Page::Vinyls { group, backroom }, true));
    }
    result
}

fn group_options(model: &Model, category: Part, group: u8) -> Vec<Item> {
    let mut options = options(Row::Part(category), model);
    let Some(catalog) = model.selected().and_then(|car| car.parts.as_ref()) else { return options };
    options.retain(|item| {
        let Some(index) = item.option.and_then(|index| index.checked_sub(1)) else { return category != Part::Vinyl };
        let Some(choice) = catalog.choices[category as usize].get(index) else { return false };
        let Some(actual) = catalog.part_groups.get(&choice.hash).copied() else { return group == 0 };
        actual == group || (category == Part::Spoiler && actual == 4)
    });
    if category == Part::Vinyl {
        let texture = VINYLS.get(group as usize).map_or(0x5577_8e5a, |(_, texture)| *texture);
        for item in &mut options {
            item.texture = texture;
        }
    }
    options
}

pub(super) fn positions(model: &Model, zone: usize) -> usize {
    let Some(catalog) = model.selected().and_then(|car| car.parts.as_ref()) else { return 0 };
    let body = model.draft.parts.parts[Part::Body as usize].or(catalog.stock[Part::Body as usize]);
    body.and_then(|body| catalog.decals[zone].body_positions.get(&body)).copied().unwrap_or(0)
}
