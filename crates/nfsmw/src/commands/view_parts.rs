//! `view-car --part paint=12 --part vinyl=30`: the car with parts of the auto shop picked by number, to look at a car's
//! paint and liveries without starting the world. The numbers are the ones the `parts` console command lists.
//! `--part plate=TEXT` puts a text on the licence plate of a car that has one (a mod car with `[customization.plate]`).

use anyhow::{Context, Result, bail};
use game_install::GameDir;
use nfsmw_data::car::customization::{Catalog, Category, Selection};

/// The category a `--part` key names.
fn category(key: &str) -> Option<Category> {
    Some(match key {
        "body" => Category::Body,
        "spoiler" => Category::Spoiler,
        "hood" => Category::Hood,
        "roof" => Category::Roof,
        "rims" => Category::Wheels,
        "paint" => Category::Paint,
        "tint" => Category::WindowTint,
        "rimpaint" => Category::RimPaint,
        "vinyl" => Category::Vinyl,
        _ => return None,
    })
}

/// The selection `parts` ask for on the car in `folder`: each is `category=number`, with the number counted from 1 as
/// the `parts` console command lists them.
pub(super) fn selection(dir: &GameDir, folder: &str, parts: &[String]) -> Result<Selection> {
    let mut selection = Selection::default();
    if parts.is_empty() {
        return Ok(selection);
    }
    let mut catalog = Catalog::load(dir, folder).with_context(|| format!("listing the parts of {folder}"))?;
    crate::garage::listing::arrange(&mut catalog, &crate::i18n::game);
    for part in parts {
        let (key, number) = part.split_once('=').with_context(|| format!("--part {part}: use category=number"))?;
        if key == "plate" {
            // The text itself, not a number: the plate keeps what its sheet can draw.
            selection.plate = Some(number.to_owned());
            continue;
        }
        let Some(category) = category(key) else {
            bail!(
                "--part {part}: the category is body, spoiler, hood, roof, rims, paint, tint, rimpaint, vinyl or plate"
            );
        };
        let choice = number
            .parse::<usize>()
            .ok()
            .and_then(|number| number.checked_sub(1))
            .and_then(|at| catalog.choices[category as usize].get(at))
            .with_context(|| format!("--part {part}: {folder} has no such {key}"))?;
        selection.parts[category as usize] = Some(choice.hash);
    }
    Ok(selection)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn categories_are_the_names_of_the_parts_command() {
        assert_eq!(category("vinyl"), Some(Category::Vinyl));
        assert_eq!(category("rimpaint"), Some(Category::RimPaint));
        assert_eq!(category("wings"), None);
    }
}
