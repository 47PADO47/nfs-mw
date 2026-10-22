//! Console counterpart of the live selectors, also useful for reproducible renderer captures.

use blackbox_gfx::RenderBackend;
use nfsmw_data::car::customization::{Category, Selection};

use super::WorldScene;

const USAGE: &str = "usage: parts [stock|<body|spoiler|hood|roof|rims> <stock|index> ...]";

pub(in crate::scenes::world) fn run(
    scene: &mut WorldScene,
    renderer: &mut dyn RenderBackend,
    args: &[&str],
) -> Result<String, String> {
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
                .map(|(i, p)| format!("{}: {}", i + 1, p.label))
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
            let category = match pair[0] {
                "body" => Category::Body,
                "spoiler" => Category::Spoiler,
                "hood" => Category::Hood,
                "roof" => Category::Roof,
                "rims" => Category::Wheels,
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
    Ok(format!("updated {car} model parts"))
}
