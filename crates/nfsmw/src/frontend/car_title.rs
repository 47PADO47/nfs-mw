//! The name of a mod car on the screens of the `Car_Select` package.
//!
//! The package names a stock car with two pictures, the maker's logo and the model's name, and hides its two name strings (they
//! hold the sample text `temp123`/`tempXXX`). A mod car has no such pictures, and the package's generic ones read `tp` and
//! `temp350`. [`pictures`] gives a mod car pictures of its own, drawn from the brand and the name its mod gave it
//! (`ui::name_picture`), which the screen sets as the textures of the two logos: the package keeps its layout and its name
//! strings stay hidden.

use blackbox_feng::fe_hash_upper;
use nfsmw_data::car::mods;

use crate::ui::{UiAssets, name_picture};

/// The size of the two picture slots (the maker's logo is 150 by 75, the model's name 256 by 64 in the package), drawn at a
/// finer size so the text stays sharp.
const MAKER_SLOT: (u32, u32) = (600, 300);
const MODEL_SLOT: (u32, u32) = (1024, 256);

/// What a screen calls the car in `folder`: the name its mod gave it, or the folder.
pub(crate) fn title(folder: &str) -> String {
    mods::display_name(folder).map_or_else(|| folder.to_owned(), str::to_owned)
}

/// The textures of the maker's logo and the model's name for the mod car in `folder`, drawn on first use; `None` for a car of
/// the game, whose pictures the package has. The maker's picture is the brand (clear for a car with none), the model's the name
/// without the brand in front of it.
pub(crate) fn pictures(assets: &UiAssets, folder: &str) -> Option<(u32, u32)> {
    if !mods::is_mod_car(folder) {
        return None;
    }
    let brand = mods::manufacturer(folder).unwrap_or_default().trim();
    let title = title(folder);
    let maker = picture(assets, "MAKER", brand, MAKER_SLOT);
    let model = picture(assets, "MODEL", model_name(&title, brand), MODEL_SLOT);
    Some((maker, model))
}

/// The name `title` gives the model: the title less the `brand` it starts with (`Tesla Roadster 2011` of `Tesla` is
/// `Roadster 2011`), or all of it when it does not start with the brand or is the brand.
fn model_name<'a>(title: &'a str, brand: &str) -> &'a str {
    if brand.is_empty() {
        return title;
    }
    let Some(head) = title.get(..brand.len()) else { return title };
    if !head.eq_ignore_ascii_case(brand) {
        return title;
    }
    let rest = title[brand.len()..].trim_start_matches([' ', '-', '_']);
    if rest.is_empty() { title } else { rest }
}

/// The texture of `text` in a slot of `size`, made when the assets do not hold it yet.
fn picture(assets: &UiAssets, slot: &str, text: &str, size: (u32, u32)) -> u32 {
    let name = format!("MODCAR_{slot}_{:08X}", fe_hash_upper(text));
    let hash = fe_hash_upper(&name);
    if assets.texture(hash).is_none() {
        assets.add_textures([name_picture::texture(&name, text, size)]);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_car_no_mod_added_is_named_by_its_folder() {
        assert_eq!(title("NOSUCHMODCAR"), "NOSUCHMODCAR");
    }

    #[test]
    fn a_car_of_the_game_has_no_pictures_of_its_own() {
        assert_eq!(pictures(&UiAssets::empty(), "NOSUCHMODCAR"), None);
    }

    #[test]
    fn the_model_is_the_title_without_the_brand_in_front() {
        assert_eq!(model_name("Tesla Roadster 2011", "Tesla"), "Roadster 2011");
        assert_eq!(model_name("tesla-Roadster", "Tesla"), "Roadster");
    }

    #[test]
    fn a_title_that_does_not_start_with_the_brand_or_is_the_brand_stays_whole() {
        assert_eq!(model_name("Roadster", "Tesla"), "Roadster");
        assert_eq!(model_name("Tesla", "Tesla"), "Tesla");
        assert_eq!(model_name("Roadster", ""), "Roadster");
    }

    #[test]
    fn a_picture_is_drawn_once() {
        let assets = UiAssets::empty();
        let first = picture(&assets, "MODEL", "Roadster", (64, 16));
        assert_eq!(picture(&assets, "MODEL", "Roadster", (64, 16)), first);
        assert_eq!(assets.late_count(), 1);
    }
}
