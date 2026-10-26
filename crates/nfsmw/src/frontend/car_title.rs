//! The name of a car on the screens of the `Car_Select` package.
//!
//! The package has two sample strings for a car's name (`temp123`, `tempXXX` in the file); the game's screens hide them and
//! name a stock car with a model-name picture instead. A mod car has no such picture, so left alone a screen would show the
//! sample text or nothing. [`show`] writes the car's title (the name its mod gave it) into the strings.

use blackbox_feng::ObjectRef;
use nfsmw_data::car::mods;

use super::logic::Cx;

/// The name hashes of the package's two car name strings (the text and its shadow).
pub(crate) const NAME_STRINGS: [u32; 2] = [0xD6D3_2016, 0x79D6_E45C];

/// What a screen calls the car in `folder`: the name its mod gave it, or the folder.
pub(crate) fn title(folder: &str) -> String {
    mods::display_name(folder).map_or_else(|| folder.to_owned(), str::to_owned)
}

/// Write the title of the mod car in `folder` into the package's name strings and show them. Nothing for a car of the game,
/// whose name the model picture carries.
pub(crate) fn show(cx: &mut Cx, folder: &str) {
    if !mods::is_mod_car(folder) {
        return;
    }
    let text = title(folder);
    let strings: Vec<ObjectRef> = cx
        .rt
        .package(cx.package)
        .into_iter()
        .flat_map(|package| package.objects.iter().enumerate())
        .filter(|(_, object)| NAME_STRINGS.contains(&object.name_hash))
        .map(|(index, _)| ObjectRef { package: cx.package, index })
        .collect();
    for object in strings {
        cx.rt.set_text(object, text.clone());
        cx.rt.set_hidden(object, false);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_car_no_mod_added_is_named_by_its_folder() {
        assert_eq!(title("NOSUCHMODCAR"), "NOSUCHMODCAR");
    }
}
