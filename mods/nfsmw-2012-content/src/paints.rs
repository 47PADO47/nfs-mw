//! The paints of a 2012 car as the `[customization]` table of its manifest.
//!
//! The 2012 game paints a car with one of the colour records its bundle carries (`chameleon_vehicle::paints`), 1 to 15
//! per car, and offers nothing else for its looks: no vinyls, rims, spoilers, tints or body kits (see
//! `docs/formats/chameleon-customization.md`). The table is therefore the car's paints and nothing more.

use chameleon_vehicle::VehiclePaint;

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

/// The `[customization]` table for `choices`, to follow the other keys of a car manifest. Empty when there is no paint,
/// which leaves the car with none of its own.
pub fn manifest_table(choices: &[PaintChoice]) -> String {
    if choices.is_empty() {
        return String::new();
    }
    let mut text = String::from("\n[customization]\ndefault_paint = 0\npaints = [\n");
    for choice in choices {
        let [r, g, b] = choice.color;
        let finish = if choice.metallic { "metallic" } else { "gloss" };
        text.push_str(&format!(
            "  {{ name = \"{}\", color = [{r}, {g}, {b}], finish = \"{finish}\" }},\n",
            choice.name()
        ));
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
        let table = manifest_table(&choices(&[paint([0.4, 0.02, 0.02], 1), paint([0.0, 0.0, 1.0], 0)]));
        let text = format!("clone = \"BMWM3GTR\"\n{table}");
        let manifest = blackbox_modding::content::parse_car("testcar", &text).unwrap();
        let own = manifest.customization.unwrap();
        assert_eq!(own.paints.len(), 2);
        assert_eq!(own.paints[0].finish, blackbox_modding::content::Finish::Metallic);
        assert_eq!(own.paints[1].color, [0, 0, 255]);
    }

    #[test]
    fn a_car_without_paints_has_no_table() {
        assert_eq!(manifest_table(&[]), "");
    }
}
