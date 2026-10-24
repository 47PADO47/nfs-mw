//! Console counterpart of the live selectors, also useful for reproducible renderer captures.

use blackbox_gfx::RenderBackend;
use nfsmw_data::car::customization::{Category, Selection};

use super::WorldScene;

const USAGE: &str = "usage: parts [stock|<body|spoiler|hood|roof|rims|paint|tint|rimpaint|vinyl> <stock|index> ...]";

pub(in crate::scenes::world) fn run(
    scene: &mut WorldScene,
    renderer: &mut dyn RenderBackend,
    args: &[&str],
) -> Result<String, String> {
    if let ["preset", name @ ..] = args {
        return preset(scene, renderer, &name.join(" "));
    }
    if scene.players.is_empty() {
        return Err("get into a car before selecting parts".into());
    }
    let car = scene.garage.applied.car.clone();
    scene.discover_garage_parts(&car).map_err(|e| format!("{e:#}"))?;
    let catalog = scene.garage.selected().and_then(|c| c.parts.as_ref()).ok_or("parts unavailable")?;
    if args.is_empty() {
        let mut lines = Vec::new();
        for category in Category::ALL {
            let options = catalog.choices[category as usize]
                .iter()
                .enumerate()
                .map(|(i, p)| format!("{}: {} [{:08X}]", i + 1, p.label, p.hash))
                .collect::<Vec<_>>()
                .join(", ");
            lines.push(format!(
                "{}: stock{}{}",
                category.label(),
                match options.is_empty() {
                    true => "",
                    false => ", ",
                },
                options
            ));
        }
        return Ok(lines.join("\n"));
    }
    let mut selection = scene.garage.applied.clone();
    if args == ["stock"] {
        selection.parts = Selection::default();
    }
    if args != ["stock"] {
        if !args.len().is_multiple_of(2) {
            return Err(USAGE.into());
        }
        for pair in args.as_chunks::<2>().0 {
            if pair[0] == "finish" {
                let (_, hash) = catalog
                    .finishes
                    .iter()
                    .find(|(label, _)| label.eq_ignore_ascii_case(pair[1]))
                    .ok_or("finish is unavailable")?;
                selection.parts.parts[Category::Paint as usize] = Some(*hash);
                continue;
            }
            if appearance(&mut selection, pair)? {
                continue;
            }
            let category = match pair[0] {
                "body" => Category::Body,
                "spoiler" => Category::Spoiler,
                "hood" => Category::Hood,
                "roof" => Category::Roof,
                "rims" => Category::Wheels,
                "paint" => Category::Paint,
                "tint" => Category::WindowTint,
                "rimpaint" => Category::RimPaint,
                "vinyl" => Category::Vinyl,
                _ => return Err(USAGE.into()),
            };
            let hash = match pair[1] {
                "stock" => None,
                value => {
                    let index = value.parse::<usize>().map_err(|_| USAGE)?;
                    let choice = index
                        .checked_sub(1)
                        .and_then(|i| catalog.choices[category as usize].get(i))
                        .ok_or("part index unavailable for this car")?;
                    Some(choice.hash)
                }
            };
            selection.parts.parts[category as usize] = hash;
        }
    }
    scene.apply_garage_selection(renderer, selection).map_err(|e| format!("{e:#}"))?;
    Ok(format!("updated {car} parts and appearance"))
}

/// Load through the same decoder and transaction as F6; never rewrite the saved preset.
fn preset(scene: &mut WorldScene, renderer: &mut dyn RenderBackend, name: &str) -> Result<String, String> {
    let load = || -> anyhow::Result<String> {
        if let Some(path) = &scene.garage.preset_picker.path {
            return crate::preset_store::load(path, name);
        }
        let path = crate::settings::Settings::config_path().ok_or_else(|| anyhow::anyhow!("No user config folder"))?;
        crate::preset_store::load(&path.with_file_name("garage-presets.toml"), name)
    };
    let payload = load().map_err(|e| format!("{e:#}"))?;
    let table: toml::Table = payload.parse().map_err(|e| format!("Invalid car preset: {e}"))?;
    let car = table.get("car").and_then(toml::Value::as_str).ok_or("Preset car is missing")?;
    scene.discover_garage_parts(car).map_err(|e| format!("{e:#}"))?;
    let selection = crate::garage::presets::decode(&payload, &scene.garage).map_err(|e| format!("{e:#}"))?;
    scene.apply_garage_selection(renderer, selection).map_err(|e| format!("{e:#}"))?;
    Ok(format!("loaded garage preset {name}"))
}

/// Color arguments are RGB hex (RRGGBB) or stock, for repeatable captures and console use.
fn appearance(selection: &mut crate::garage::Selection, pair: &[&str; 2]) -> Result<bool, String> {
    if pair[0] == "gauges" {
        selection.gauges.skin = pair[1].parse::<u8>().map_err(|_| "gauge style must be 0..10")?;
        return Ok(true);
    }
    let color = match pair[0] {
        "bodycolor" => &mut selection.color,
        "rimcolor" => &mut selection.parts.rim_color,
        "vinylcolor1" => &mut selection.parts.vinyl_colors[0],
        "vinylcolor2" => &mut selection.parts.vinyl_colors[1],
        "vinylcolor3" => &mut selection.parts.vinyl_colors[2],
        "gaugebacking" => &mut selection.gauges.backing,
        "gaugeneedle" => &mut selection.gauges.needle,
        "gaugedigits" => &mut selection.gauges.digits,
        _ => return Ok(false),
    };
    if pair[1] == "stock" {
        *color = None;
        clear_color_identity(selection, pair[0]);
        return Ok(true);
    }
    let hex = pair[1].trim_start_matches('#');
    if hex.len() != 6 || !hex.is_ascii() {
        return Err("color must be RRGGBB or stock".into());
    }
    let value = u32::from_str_radix(hex, 16).map_err(|_| "color must be RRGGBB or stock")?;
    *color = Some([(value >> 16) as u8, (value >> 8) as u8, value as u8]);
    clear_color_identity(selection, pair[0]);
    Ok(true)
}

fn clear_color_identity(selection: &mut crate::garage::Selection, key: &str) {
    match key {
        "vinylcolor1" => selection.parts.vinyl_color_parts[0] = None,
        "vinylcolor2" => selection.parts.vinyl_color_parts[1] = None,
        "vinylcolor3" => selection.parts.vinyl_color_parts[2] = None,
        "gaugebacking" => selection.gauges.color_parts[0] = None,
        "gaugeneedle" => selection.gauges.color_parts[1] = None,
        "gaugedigits" => selection.gauges.color_parts[2] = None,
        _ => {}
    }
}

#[cfg(test)]
mod color_identity_tests {
    use super::*;

    #[test]
    fn console_rgb_edit_clears_native_identity_without_affecting_other_channels() {
        let mut selection = crate::garage::Selection::default();
        selection.parts.vinyl_color_parts = [Some(10), Some(11), Some(12)];
        selection.gauges.color_parts = [Some(20), Some(21), Some(22)];
        appearance(&mut selection, &["vinylcolor1", "010203"]).unwrap();
        appearance(&mut selection, &["gaugedigits", "040506"]).unwrap();
        assert_eq!(selection.parts.vinyl_color_parts, [None, Some(11), Some(12)]);
        assert_eq!(selection.gauges.color_parts, [Some(20), Some(21), None]);
        assert_eq!(selection.parts.vinyl_colors[0], Some([1, 2, 3]));
        assert_eq!(selection.gauges.digits, Some([4, 5, 6]));
    }
}
