//! The paints and liveries of a 2012 car as the `[customization]` table of its manifest.
//!
//! The 2012 game paints a car with one of the colour records its bundle carries (`chameleon_vehicle::paints`), 1 to 15
//! per car, and offers nothing else for its looks but the liveries of the car's other bundles: no rims, spoilers, tints
//! or body kits (see `docs/formats/chameleon-customization.md`). The table is therefore the car's paints and its
//! liveries, if it has any.

use chameleon_vehicle::VehiclePaint;

use crate::liveries::LiveryChoice;

/// A paint the car is offered in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PaintChoice {
    /// sRGB.
    pub color: [u8; 3],
    pub metallic: bool,
}

impl PaintChoice {
    /// The name the shop shows, made of the finish and the colour, which the 2012 records do not name.
    pub fn name(&self) -> String {
        let [r, g, b] = self.color;
        let finish = if self.metallic { "Metallic" } else { "Gloss" };
        format!("{finish} #{r:02X}{g:02X}{b:02X}")
    }
}

/// The distinct paints of `paints`, in the order of the bundle. Two records of one colour and finish are one paint (the
/// 2012 bundles repeat a paint under two ids).
pub fn choices(paints: &[VehiclePaint]) -> Vec<PaintChoice> {
    let mut out: Vec<PaintChoice> = Vec::with_capacity(paints.len());
    for paint in paints {
        let choice = PaintChoice { color: paint.srgb(), metallic: paint.is_metallic() };
        if !out.contains(&choice) {
            out.push(choice);
        }
    }
    out
}

/// `paints` with the ones the vehicle's paint pack names first, in the pack's order, then the others in the order of the
/// bundle. The pack is the shop's list, and its first colour is taken as the car's default paint (that this is the 2012
/// game's choice is not confirmed); `order` holds the `VehiclePaint` ids of the pack's plain colours.
pub fn in_pack_order(paints: &[VehiclePaint], order: &[u32]) -> Vec<VehiclePaint> {
    let mut taken = vec![false; paints.len()];
    let mut out = Vec::with_capacity(paints.len());
    for id in order {
        let Some(at) = paints.iter().zip(&taken).position(|(paint, done)| paint.id == *id && !*done) else { continue };
        taken[at] = true;
        out.push(paints[at]);
    }
    out.extend(paints.iter().zip(&taken).filter(|(_, done)| !**done).map(|(paint, _)| *paint));
    out
}

/// The `[customization]` table for `choices` and `liveries`, to follow the other keys of a car manifest. Empty when there
/// is no paint, which leaves the car with none of its own (a table needs a paint). `dressed` is the position of the livery
/// the car wears until the player changes it (its first, or a police car's markings).
pub fn manifest_table(choices: &[PaintChoice], liveries: &[LiveryChoice], dressed: Option<usize>) -> String {
    if choices.is_empty() {
        return String::new();
    }
    let mut text = String::from("\n[customization]\ndefault_paint = 0\n");
    if let Some(position) = dressed.filter(|position| *position < liveries.len()) {
        text.push_str(&format!("default_livery = {position}\n"));
    }
    text.push_str("paints = [\n");
    for choice in choices {
        let [r, g, b] = choice.color;
        let finish = if choice.metallic { "metallic" } else { "gloss" };
        text.push_str(&format!(
            "  {{ name = \"{}\", color = [{r}, {g}, {b}], finish = \"{finish}\" }},\n",
            choice.name()
        ));
    }
    text.push_str("]\n");
    if liveries.is_empty() {
        return text;
    }
    text.push_str("liveries = [\n");
    for livery in liveries {
        text.push_str(&format!("  {{ name = \"{}\", texture = \"{}\" }},\n", livery.name, livery.texture));
    }
    text.push_str("]\n");
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    fn paint(diffuse: [f32; 3], kind: u16) -> VehiclePaint {
        VehiclePaint { id: 1, diffuse, flake: [0.0; 3], secondary: [0.0; 3], kind, swatch: 0 }
    }

    #[test]
    fn repeated_colours_are_one_paint_and_the_order_is_kept() {
        let paints = [paint([1.0, 0.0, 0.0], 0), paint([0.0, 1.0, 0.0], 1), paint([1.0, 0.0, 0.0], 0)];
        let found = choices(&paints);
        assert_eq!(found.len(), 2);
        assert_eq!(found[0], PaintChoice { color: [255, 0, 0], metallic: false });
        assert!(found[1].metallic);
    }

    #[test]
    fn the_same_colour_in_two_finishes_is_two_paints_with_two_names() {
        let found = choices(&[paint([1.0, 0.0, 0.0], 0), paint([1.0, 0.0, 0.0], 1)]);
        assert_eq!(found.len(), 2);
        assert_ne!(found[0].name(), found[1].name());
        assert_eq!(found[0].name(), "Gloss #FF0000");
    }

    #[test]
    fn the_table_is_valid_and_the_game_accepts_it() {
        let table = manifest_table(&choices(&[paint([0.4, 0.02, 0.02], 1), paint([0.0, 0.0, 1.0], 0)]), &[], None);
        let text = format!("name = \"Test car\"\n{table}");
        let manifest = blackbox_modding::content::parse_car("testcar", &text).unwrap();
        let own = manifest.customization.unwrap();
        assert_eq!(own.paints.len(), 2);
        assert_eq!(own.paints[0].finish, blackbox_modding::content::Finish::Metallic);
        assert_eq!(own.paints[1].color, [0, 0, 255]);
    }

    #[test]
    fn a_car_without_paints_has_no_table() {
        assert_eq!(manifest_table(&[], &[], None), "");
        let livery = LiveryChoice { name: "Livery 1".into(), texture: "X_LIV1".into() };
        assert_eq!(manifest_table(&[], &[livery], None), "", "a table needs a paint, and liveries do not make one");
    }

    #[test]
    fn liveries_follow_the_paints_and_the_game_accepts_them() {
        let liveries = [crate::liveries::choice("TESTCAR", 0), crate::liveries::choice("TESTCAR", 1)];
        let table = manifest_table(&choices(&[paint([0.4, 0.02, 0.02], 0)]), &liveries, None);
        let text = format!("name = \"Test car\"\n{table}");
        let own = blackbox_modding::content::parse_car("testcar", &text).unwrap().customization.unwrap();
        assert_eq!(own.paints.len(), 1);
        assert_eq!(own.liveries.len(), 2);
        assert_eq!(own.liveries[1].name, "Livery 2");
        assert_eq!(own.liveries[1].texture, "TESTCAR_LIV2");
    }

    #[test]
    fn a_car_without_liveries_writes_no_livery_key() {
        let table = manifest_table(&choices(&[paint([0.4, 0.02, 0.02], 0)]), &[], None);
        assert!(!table.contains("liveries"));
    }

    #[test]
    fn a_car_can_wear_one_of_its_liveries() {
        let liveries = [crate::liveries::choice("TESTCAR", 0), crate::liveries::choice("TESTCAR", 1)];
        let paints = choices(&[paint([0.4, 0.02, 0.02], 0)]);
        let table = manifest_table(&paints, &liveries, Some(1));
        let text = format!(
            "name = \"Test car\"
{table}"
        );
        let own = blackbox_modding::content::parse_car("testcar", &text).unwrap().customization.unwrap();
        assert_eq!(own.default_livery, Some(1));
        assert!(!manifest_table(&paints, &liveries, None).contains("default_livery"));
        assert!(!manifest_table(&paints, &liveries, Some(2)).contains("default_livery"), "past the end");
        assert!(!manifest_table(&paints, &[], Some(0)).contains("default_livery"), "no liveries");
    }

    fn paint_with_id(id: u32, diffuse: [f32; 3]) -> VehiclePaint {
        VehiclePaint { id, ..paint(diffuse, 0) }
    }

    #[test]
    fn the_packs_colours_come_first_in_the_packs_order_and_the_rest_keep_the_bundles() {
        let paints = [
            paint_with_id(1, [1.0, 0.0, 0.0]),
            paint_with_id(2, [0.0, 1.0, 0.0]),
            paint_with_id(3, [0.0, 0.0, 1.0]),
            paint_with_id(4, [1.0, 1.0, 0.0]),
        ];
        let ids = |found: Vec<VehiclePaint>| found.iter().map(|paint| paint.id).collect::<Vec<_>>();
        assert_eq!(ids(in_pack_order(&paints, &[3, 1])), vec![3, 1, 2, 4]);
        assert_eq!(ids(in_pack_order(&paints, &[])), vec![1, 2, 3, 4], "no pack: the order of the bundle");
        assert_eq!(ids(in_pack_order(&paints, &[9, 2, 2])), vec![2, 1, 3, 4], "unknown and repeated ids are skipped");
    }
}
