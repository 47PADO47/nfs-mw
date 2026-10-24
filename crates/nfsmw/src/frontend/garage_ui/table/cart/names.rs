//! Original localized cart categories and part names, independent of diagnostic row labels.

use crate::garage::{Model, Selection, menu_rows::Row};
use crate::ui::Text;
use nfsmw_data::car::customization::{Category as Part, Choice};
use nfsmw_data::car::physics::PerformanceCategory as Hardware;

pub(super) fn label(row: Row, model: &Model, selected: &Selection) -> String {
    let Some(car) = model.cars.iter().find(|car| car.folder == selected.car) else { return String::new() };
    let Some(catalog) = car.parts.as_ref() else { return String::new() };
    let category = match row {
        Row::Part(part) => game(part_label(part), part_fallback(part)),
        Row::Gauge => game(0x7898_0a6b, "Custom Gauges"),
        Row::Number(_) => game(0x6857_e5ac, "Numbers"),
        Row::Decal(_, _) => game(0x9559_80bc, "Decals"),
        Row::Performance(hardware) | Row::Junkman(hardware) => {
            game(performance_label(hardware, car.performance_details.turbo), hardware.label())
        }
        _ => row.label(),
    };
    let stock = || game(0x60a6_62f5, "Stock");
    let detail = match row {
        Row::Part(part @ (Part::Paint | Part::RimPaint)) => {
            let hash = selected.parts.parts[part as usize].or(catalog.stock[part as usize]);
            match hash.and_then(|hash| catalog.paint_names.get(&hash)) {
                Some((finish, color)) => {
                    let finish = match part {
                        Part::RimPaint => 0xb310_0a3e,
                        _ => *finish,
                    };
                    let finish = game(finish, "");
                    let color = game(*color, "");
                    format!("{finish} {color}").trim().to_owned()
                }
                None => stock(),
            }
        }
        Row::Part(part) => {
            let choice = selected.parts.parts[part as usize]
                .and_then(|hash| catalog.choices[part as usize].iter().find(|choice| choice.hash == hash));
            let native = choice.and_then(|choice| catalog.native_names.get(&choice.hash));
            let name = native
                .map(|(name, language)| {
                    language.and_then(|label| Text::Game(label).resolve()).unwrap_or_else(|| name.clone())
                })
                .or_else(|| choice.map(choice_name))
                .unwrap_or_else(stock);
            match (part, choice) {
                (Part::Wheels, Some(choice)) => match catalog.rim_metadata.get(&choice.hash) {
                    Some((_, radius)) => {
                        let name = native
                            .map(|(name, _)| {
                                let end = name.char_indices().rev().nth(5).map(|(at, _)| at).unwrap_or(name.len());
                                name[..end].to_owned()
                            })
                            .unwrap_or(name);
                        format!("{name} {radius}\"")
                    }
                    None => name,
                },
                (_, Some(choice)) if choice.carbon => format!("{} {name}", game(0x5415_b874, "Carbon")),
                _ => name,
            }
        }
        Row::Gauge => catalog.gauge_names.get(&selected.gauges.skin).map(choice_name).unwrap_or_else(stock),
        Row::Number(_) => {
            let digits = [0, 1].map(|digit| {
                selected.parts.decals[3][6 + digit]
                    .and_then(|hash| catalog.number_choices[digit].iter().find(|choice| choice.hash == hash))
            });
            match digits {
                [Some(first), Some(second)] => format!("{}{}", choice_name(first), choice_name(second)),
                _ => game(0xbe43_4a38, "Remove"),
            }
        }
        Row::Decal(zone, position) => {
            let zone_label = [0x301d_edd3, 0x48e6_ca49, 0x3436_7c86, 0xddf8_0259, 0x8a76_97d6, 0xb1f9_b0c9][zone];
            let location = game(zone_label, nfsmw_data::car::customization::DecalZone::ALL[zone].label());
            let choice = selected.parts.decals[zone][position]
                .and_then(|hash| catalog.decals[zone].choices.iter().find(|choice| choice.hash == hash));
            let Some(choice) = choice else {
                return format!("{category}: {location} - {}", game(0x7177_dc17, "Remove"));
            };
            let part = choice_name(choice);
            match zone {
                2 | 3 => format!(
                    "{location} {} {part}",
                    game(0x7d21_2cfa + position as u32, &format!("Slot {}", position + 1))
                ),
                _ => format!("{location} {part}"),
            }
        }
        Row::Performance(hardware) => {
            let selected = selected.performance.get(hardware);
            let max = car.performance.categories[hardware as usize].max_level;
            let global = match selected {
                0 => 0,
                _ => 6u8.saturating_sub(max).saturating_add(selected),
            };
            game(0x69c2_70c3 + u32::from(global), "Stock")
        }
        Row::Junkman(_) => game(0xedd1_4807, "Junkman"),
        _ => String::new(),
    };
    format!("{category}: {detail}")
}

fn game(label: u32, fallback: &str) -> String {
    Text::Game(label).resolve().unwrap_or_else(|| fallback.to_owned())
}

fn choice_name(choice: &Choice) -> String {
    choice.language_hash.and_then(|hash| Text::Game(hash).resolve()).unwrap_or_else(|| choice.label.clone())
}

fn part_label(part: Part) -> u32 {
    match part {
        Part::Body => 0x6134_c218,
        Part::Spoiler => 0x94e7_3021,
        Part::Hood => 0x04d4_a88d,
        Part::Roof => 0x61e8_f83c,
        Part::Wheels => 0xf868_eb0b,
        Part::Paint => 0x055d_a70c,
        Part::WindowTint => 0xd327_29a6,
        Part::RimPaint => 0xe126_ff53,
        Part::Vinyl => 0xbfa5_2c55,
    }
}

fn part_fallback(part: Part) -> &'static str {
    match part {
        Part::Paint => "Paint",
        Part::RimPaint => "Rim Paint",
        Part::Vinyl => "Vinyls",
        _ => part.label(),
    }
}

fn performance_label(category: Hardware, turbo: bool) -> u32 {
    match category {
        Hardware::Tires => 0x05aa_9137,
        Hardware::Brakes => 0x9199_7ee8,
        Hardware::Chassis => 0x6e10_1aa7,
        Hardware::Transmission => 0x29aa_74ba,
        Hardware::Engine => 0x9853_d9a6,
        Hardware::Nitrous => 0x4ce1_9aa4,
        Hardware::Induction if turbo => 0x05b1_255c,
        Hardware::Induction => 0xbb68_12bb,
    }
}
