//! Bridge between installed part identities and the original stable record.
use anyhow::{Context, Result, ensure};
use nfsmw_career::stable::CustomizationRecord;
use nfsmw_data::car::{
    CarTables, LAYOUT, Paint, PresetRide,
    customization::{Category, DecalZone},
    slot, stock_parts,
};

use crate::garage::{Model, Selection};

mod colors;

// Installed CUSTOM_HUD parts use this integer attribute for the style (0–10).
const HUD_STYLE: u32 = 0x3111_51f8;

fn saved_hash(record: &CustomizationRecord, tables: &CarTables, car: u32, slot: usize) -> Result<Option<u32>> {
    let Some(index) = record.part(slot) else { return Ok(None) };
    let part = tables.parts.parts().get(usize::from(index)).context("Saved part index is unavailable")?;
    ensure!(
        part.part_id == LAYOUT.slots[slot].part_id
            && tables.slot_types.search_types(slot, car).any(|kind| kind == tables.parts.type_name_hash(part)),
        "Saved part is incompatible with its car or slot"
    );
    Ok(Some(part.name_hash))
}

fn store_hash(record: &mut CustomizationRecord, tables: &CarTables, slot: usize, hash: Option<u32>) -> Result<()> {
    let index = hash
        .map(|hash| {
            let index = tables
                .parts
                .parts()
                .iter()
                .position(|part| part.name_hash == hash)
                .context("Part is absent from the installed database")?;
            u16::try_from(index).context("Part index is too large for an original save")
        })
        .transpose()?;
    ensure!(record.set_part(slot, index), "Part index cannot be stored in an original save");
    Ok(())
}

fn read_color(
    record: &CustomizationRecord,
    tables: &CarTables,
    car: u32,
    slot: usize,
    stock: Option<u32>,
) -> Result<(Option<[u8; 3]>, Option<u32>)> {
    colors::read(record, slot, stock, &color_swatches(tables, car, slot)?)
}

fn color_swatches(tables: &CarTables, car: u32, slot: usize) -> Result<Vec<colors::Swatch>> {
    let types: Vec<_> = tables.slot_types.search_types(slot, car).collect();
    tables
        .parts
        .parts()
        .iter()
        .enumerate()
        .filter(|(_, part)| {
            part.part_id == LAYOUT.slots[slot].part_id && types.contains(&tables.parts.type_name_hash(part))
        })
        .map(|(index, part)| {
            Ok(colors::Swatch {
                hash: part.name_hash,
                index: u16::try_from(index).context("Color part index is too large for an original save")?,
                rgb: Paint::of(tables, part).rgb,
            })
        })
        .collect()
}

fn write_color(
    record: &mut CustomizationRecord,
    tables: &CarTables,
    car: u32,
    slot: usize,
    stock: Option<u32>,
    color: Option<[u8; 3]>,
    native_hash: Option<u32>,
) -> Result<()> {
    colors::write(record, slot, stock, color, native_hash, &color_swatches(tables, car, slot)?)
}

pub(super) fn read(record: &CustomizationRecord, tables: &CarTables, folder: &str, model: &Model) -> Result<Selection> {
    let entry = model.cars.iter().find(|car| car.folder == folder).context("Saved car is not installed")?;
    let catalog = entry.parts.as_ref().context("Saved car part catalog is unavailable")?;
    let car = tables.car_type_for_folder(folder).context("Saved car type is unavailable")?;
    let stock = stock_parts(tables, car);
    let mut selection = Selection { car: folder.to_owned(), ..Default::default() };
    // Older remake car purchases allocated a zero-filled record, before the native codec existed.
    if record.as_bytes().iter().all(|byte| *byte == 0) {
        return Ok(selection);
    }
    for category in Category::ALL {
        let Some(hash) = saved_hash(record, tables, car.type_name_hash, category.slot())? else { continue };
        if Some(hash) == catalog.stock[category as usize] {
            continue;
        }
        selection.parts.parts[category as usize] = Some(hash);
    }
    for zone in DecalZone::ALL {
        for (position, slot) in selection.parts.decals[zone as usize].iter_mut().enumerate().take(8) {
            *slot = saved_hash(record, tables, car.type_name_hash, zone.texture_slot(position))?;
        }
    }
    for i in 0..3 {
        (selection.parts.vinyl_colors[i], selection.parts.vinyl_color_parts[i]) =
            read_color(record, tables, car.type_name_hash, 79 + i, stock[79 + i].map(|p| p.name_hash))?;
    }
    if let Some(hash) = saved_hash(record, tables, car.type_name_hash, 132)? {
        let part = tables.parts.by_name_hash(hash).context("Saved gauge style is unavailable")?;
        let style = tables
            .parts
            .attributes(part)
            .find(|(key, _)| *key == HUD_STYLE)
            .context("Saved gauge style has no installed identity")?
            .1;
        selection.gauges.skin = u8::try_from(style).context("Saved gauge style is invalid")?;
    }
    let colors: Vec<_> = (133..=135)
        .map(|slot| read_color(record, tables, car.type_name_hash, slot, stock[slot].map(|p| p.name_hash)))
        .collect::<Result<_>>()?;
    [selection.gauges.backing, selection.gauges.needle, selection.gauges.digits] =
        std::array::from_fn(|index| colors[index].0);
    selection.gauges.color_parts = std::array::from_fn(|index| colors[index].1);
    for (i, level) in record.performance().into_iter().enumerate() {
        ensure!(
            level >= 0 && level <= i32::from(entry.performance.categories[i].max_level),
            "Saved performance stage is unavailable"
        );
        selection.performance.levels[i] = level as u8;
    }
    ensure!(record.junkman() < 128, "Saved Junkman mask is invalid");
    selection.performance.junkman = record.junkman() as u8;
    for category in nfsmw_data::car::physics::PerformanceCategory::ALL {
        ensure!(
            !selection.performance.has_junkman(category)
                || entry.performance.junkman_available(category, selection.performance),
            "Saved Junkman package is unavailable"
        );
    }
    ensure!((0..3).contains(&record.active_tuning()), "Saved tuning profile is invalid");
    selection.tuning.active = record.active_tuning() as u8;
    for (p, values) in record.tuning().into_iter().enumerate() {
        for (i, value) in values.into_iter().enumerate() {
            ensure!(value.is_finite() && (-1.0..=1.0).contains(&value), "Saved tuning value is invalid");
            selection.tuning.profiles[p][i] = (value * 5.0).round() as i8;
        }
    }
    catalog.validate(&selection.parts)?;
    selection.gauges.validate(catalog)?;
    Ok(selection)
}

pub(super) fn write(
    record: &CustomizationRecord,
    tables: &CarTables,
    selection: &Selection,
) -> Result<CustomizationRecord> {
    let mut record = match record.as_bytes().iter().all(|byte| *byte == 0) {
        true => CustomizationRecord::default(),
        false => record.clone(),
    };
    let car = tables.car_type_for_folder(&selection.car).context("Car type is unavailable")?;
    let stock = stock_parts(tables, car);
    let current_body =
        saved_hash(&record, tables, car.type_name_hash, slot::BODY)?.or(stock[slot::BODY].map(|p| p.name_hash));
    let selected_body = selection.parts.parts[Category::Body as usize].or(stock[slot::BODY].map(|p| p.name_hash));
    let current_wheel = saved_hash(&record, tables, car.type_name_hash, slot::FRONT_WHEEL)?
        .or(stock[slot::FRONT_WHEEL].map(|p| p.name_hash));
    let selected_wheel =
        selection.parts.parts[Category::Wheels as usize].or(stock[slot::FRONT_WHEEL].map(|p| p.name_hash));
    for category in Category::ALL {
        store_hash(&mut record, tables, category.slot(), selection.parts.parts[category as usize])?;
    }
    if current_wheel != selected_wheel {
        store_hash(&mut record, tables, slot::REAR_WHEEL, selection.parts.parts[Category::Wheels as usize])?;
    }
    if current_body != selected_body {
        let mut parts = vec![PresetRide::STOCK; LAYOUT.slots.len()];
        parts[slot::BODY] = selected_body.unwrap_or(PresetRide::EMPTY);
        let ride = PresetRide {
            car_type_name: car.type_name.clone(),
            preset_name: String::new(),
            fe_key: 0,
            vehicle_key: 0,
            parts,
        };
        let resolved = nfsmw_data::car::preset_parts(tables, car, &ride);
        for slot in slot::DAMAGE0.chain(slot::KIT_DECALS) {
            store_hash(&mut record, tables, slot, resolved[slot].map(|p| p.name_hash))?;
        }
    }
    for zone in DecalZone::ALL {
        for (position, decal) in selection.parts.decals[zone as usize].iter().enumerate().take(8) {
            store_hash(&mut record, tables, zone.texture_slot(position), *decal)?;
        }
    }
    for (i, color) in selection.parts.vinyl_colors.into_iter().enumerate() {
        write_color(
            &mut record,
            tables,
            car.type_name_hash,
            79 + i,
            stock[79 + i].map(|p| p.name_hash),
            color,
            selection.parts.vinyl_color_parts[i],
        )?;
    }
    let style = tables
        .parts
        .parts()
        .iter()
        .find(|part| {
            part.part_id == 82
                && tables
                    .slot_types
                    .search_types(132, car.type_name_hash)
                    .any(|kind| kind == tables.parts.type_name_hash(part))
                && tables
                    .parts
                    .attributes(part)
                    .any(|(key, value)| key == HUD_STYLE && value == u32::from(selection.gauges.skin))
        })
        .context("Gauge style is absent from the installed database")?;
    store_hash(&mut record, tables, 132, Some(style.name_hash))?;
    for (i, color) in selection.gauges.colors().into_iter().enumerate() {
        write_color(
            &mut record,
            tables,
            car.type_name_hash,
            133 + i,
            stock[133 + i].map(|p| p.name_hash),
            color,
            selection.gauges.color_parts[i],
        )?;
    }
    selection.tuning.validate()?;
    ensure!(selection.performance.junkman & 128 == 0, "Invalid Junkman category");
    record.set_performance(selection.performance.levels.map(i32::from), u32::from(selection.performance.junkman));
    record.set_tuning(
        selection.tuning.profiles.map(|profile| profile.map(|value| f32::from(value) / 5.0)),
        i32::from(selection.tuning.active),
    );
    Ok(record)
}

#[cfg(test)]
#[path = "records_tests.rs"]
mod tests;

/// The extension is accepted only for the exact native record it accompanied.
pub(super) fn fingerprint(record: &CustomizationRecord) -> String {
    save_store::digest::md5(record.as_bytes()).iter().map(|byte| format!("{byte:02x}")).collect()
}
