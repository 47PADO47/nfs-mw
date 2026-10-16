//! The reflection options beyond the sizes and the mirror tint: the reach of the captures, the car cube's detail and
//! content, and the vehicles' environment brightness. The values and their text rules are in `blackbox_gfx`; this
//! file holds the menu steps, the labels and the config file's forms, as `reflection_res.rs` does for the sizes.

use std::str::FromStr;

use blackbox_gfx::reflection::brightness::VehicleBrightness;
use blackbox_gfx::reflection::content::ReflectionContent;
use blackbox_gfx::reflection::distance::ReflectionDistance;
use blackbox_gfx::reflection::lod::ReflectionLod;
use toml::Value;

use super::Partial;

/// The four options, as the settings hold them. The default is the port's current picture.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ReflectionExtra {
    pub distance: ReflectionDistance,
    pub lod: ReflectionLod,
    pub content: ReflectionContent,
    pub brightness: VehicleBrightness,
}

impl ReflectionExtra {
    /// The values the partial sets, and the default for each it does not.
    pub fn from_partial(p: &Partial) -> Self {
        Self {
            distance: p.reflection_distance.unwrap_or_default(),
            lod: p.reflection_lod.unwrap_or_default(),
            content: p.reflection_content.unwrap_or_default(),
            brightness: p.vehicle_reflection_brightness.unwrap_or_default(),
        }
    }
}

/// The brightness steps of the Video options, in order. A value from the config file that is not in this list steps
/// from the first entry.
pub const MENU_BRIGHTNESS: [VehicleBrightness; 5] = [
    VehicleBrightness(0.5),
    VehicleBrightness(1.0),
    VehicleBrightness(1.5),
    VehicleBrightness(2.0),
    VehicleBrightness(3.0),
];

/// The next (or previous) entry of `all`, wrapping. An entry that is not in `all` steps from the first.
pub fn cycle<T: Copy + PartialEq>(all: &[T], current: T, forward: bool) -> T {
    let at = all.iter().position(|v| *v == current).unwrap_or(0);
    let next = match forward {
        true => (at + 1) % all.len(),
        false => (at + all.len() - 1) % all.len(),
    };
    all[next]
}

/// The next (or previous) brightness in [`MENU_BRIGHTNESS`], wrapping.
pub fn step_brightness(current: VehicleBrightness, forward: bool) -> VehicleBrightness {
    cycle(&MENU_BRIGHTNESS, current, forward)
}

/// The row's text for the reach: `Unlimited` or `Original`.
pub fn distance_label(distance: ReflectionDistance) -> &'static str {
    match distance {
        ReflectionDistance::Unlimited => "Unlimited",
        ReflectionDistance::Original => "Original",
    }
}

/// The row's text for the detail rule: `Capture` or `Main View`.
pub fn lod_label(lod: ReflectionLod) -> &'static str {
    match lod {
        ReflectionLod::Capture => "Capture",
        ReflectionLod::View => "Main View",
    }
}

/// The row's text for the content: `Original` or `Restored`.
pub fn content_label(content: ReflectionContent) -> &'static str {
    match content {
        ReflectionContent::Original => "Original",
        ReflectionContent::Restored => "Restored",
    }
}

/// A brightness in the config file: text (`100%`, `150`) or a whole number of percent.
pub fn brightness_from_toml(value: &Value) -> Result<VehicleBrightness, String> {
    match value {
        Value::Integer(percent) => VehicleBrightness::from_str(&percent.to_string()),
        other => VehicleBrightness::from_str(other.as_str().ok_or("expected a percentage or a whole number")?),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::{Partial, Settings, env, file, write};

    #[test]
    fn the_defaults_keep_the_port_picture() {
        let s = Settings::from(Partial::default());
        assert_eq!(s.reflection_extra, ReflectionExtra::default());
        let g = s.graphics();
        assert_eq!(g.reflection_distance, ReflectionDistance::Unlimited);
        assert_eq!(g.reflection_lod, ReflectionLod::Capture);
        assert_eq!(g.reflection_content, ReflectionContent::Original);
        assert_eq!(g.vehicle_reflection_brightness, VehicleBrightness::DEFAULT);
    }

    #[test]
    fn the_menu_cycles_the_choices_and_steps_the_brightness_in_its_list() {
        assert_eq!(cycle(&ReflectionDistance::ALL, ReflectionDistance::Unlimited, true), ReflectionDistance::Original);
        assert_eq!(cycle(&ReflectionDistance::ALL, ReflectionDistance::Original, true), ReflectionDistance::Unlimited);
        assert_eq!(cycle(&ReflectionLod::ALL, ReflectionLod::Capture, false), ReflectionLod::View);
        assert_eq!(cycle(&ReflectionContent::ALL, ReflectionContent::Original, true), ReflectionContent::Restored);
        let mut b = VehicleBrightness::DEFAULT;
        b = step_brightness(b, true);
        assert_eq!(b, VehicleBrightness(1.5));
        b = step_brightness(b, false);
        assert_eq!(b, VehicleBrightness::DEFAULT);
        assert_eq!(step_brightness(VehicleBrightness(3.0), true), VehicleBrightness(0.5), "wraps");
        assert_eq!(
            step_brightness(VehicleBrightness(0.8), true),
            VehicleBrightness(1.0),
            "off-list value steps from the start"
        );
    }

    #[test]
    fn the_rows_show_their_names() {
        assert_eq!(distance_label(ReflectionDistance::Original), "Original");
        assert_eq!(lod_label(ReflectionLod::View), "Main View");
        assert_eq!(content_label(ReflectionContent::Restored), "Restored");
        assert_eq!(VehicleBrightness(2.0).to_string(), "200%");
    }

    #[test]
    fn the_config_file_round_trips_text_and_whole_numbers() {
        let changes = Partial {
            reflection_distance: Some(ReflectionDistance::Original),
            reflection_lod: Some(ReflectionLod::View),
            reflection_content: Some(ReflectionContent::Restored),
            vehicle_reflection_brightness: Some(VehicleBrightness(1.5)),
            ..Partial::default()
        };
        let text = write::merge("", &changes).unwrap();
        assert!(text.contains("reflection_distance = \"original\""), "{text}");
        assert!(text.contains("vehicle_reflection_brightness = \"150%\""), "{text}");
        assert_eq!(file::parse(&text, "test"), changes);
        let whole = file::parse("vehicle_reflection_brightness = 200", "test");
        assert_eq!(whole.vehicle_reflection_brightness, Some(VehicleBrightness(2.0)));
    }

    #[test]
    fn a_bad_value_in_the_file_falls_back_and_the_environment_overrides_it() {
        let bad = file::parse("reflection_distance = \"far\"\nvehicle_reflection_brightness = 400", "test");
        assert_eq!((bad.reflection_distance, bad.vehicle_reflection_brightness), (None, None));
        let saved = file::parse("reflection_lod = \"capture\"", "test");
        let from_env = env::read(|key| (key == env::REFLECTION_LOD).then(|| "view".to_owned()));
        let s = Settings::from(from_env.or(saved));
        assert_eq!(s.reflection_extra.lod, ReflectionLod::View, "the environment wins over the file");
    }
}
