//! The text of the sky variant's profile and of its replacement set.

use super::plan::{DOMES, DomeSpec, WeatherSpec};
use super::{FOLDER, LABEL_ID, MOON_FILE, VALUE_LABEL_ID, VARIANT, dome_file};

/// How bright the 2012 clouds are drawn in the 2005 game's SDR sky **[our choice, looked at]**: the data's tints reach 1.9.
pub const EXPOSURE: f32 = 0.55;
/// Half the moon picture's width as an angle: the picture's disc fills three quarters of it, so 0.0173 gives the disc the
/// 0.013 radians (0.75 degrees) the game's own moon has.
pub const MOON_SIZE: f32 = 0.0173;

/// The profile (`docs/modding/sky-variants.md`) for the domes in `written` (the ones whose picture was written) and the weathers
/// whose keys all name them.
pub fn profile_text(weathers: &[WeatherSpec], written: &[DomeSpec], moon: bool) -> String {
    let mut text = format!("format = 1\nexposure = {EXPOSURE}\n");
    if moon {
        text.push_str(&format!("\n[moon]\nfile = \"{FOLDER}/{MOON_FILE}\"\nsize = {MOON_SIZE}\n"));
    }
    for dome in DOMES.iter().filter(|dome| written.iter().any(|w| w.key == dome.key)) {
        text.push_str(&format!("\n[[dome]]\nname = \"{}\"\nfile = \"{FOLDER}/{}\"\n", dome.name, dome_file(dome.name)));
        if let Some(glow) = dome.glow {
            text.push_str(&format!("glow = {glow}\n"));
        }
    }
    for weather in weathers {
        text.push_str(&format!("\n[[weather]]\nmax_cover = {}\ngain = {}\n", weather.max_cover, weather.gain));
        for key in &weather.keys {
            let [r, g, b] = key.tint;
            text.push_str(&format!(
                "\n[[weather.key]]\nhour = {:?}\ndome = \"{}\"\ntint = [{r:.4}, {g:.4}, {b:.4}]\n",
                key.hour, key.dome
            ));
        }
    }
    text
}

/// The replacement set (`replace/sky-mw2012.toml`, `docs/modding/api-replace.md`) that declares the value of the `sky` kind. It adds
/// the moon picture to the world's textures, which makes it a set the host accepts; the pictures the sky draws are the profile's.
pub fn replace_manifest(moon: bool) -> String {
    let mut text = format!(
        "format = 1\n\n[set]\nkind = \"sky\"\nkind_label = \"{LABEL_ID}\"\nvalue = \"{VARIANT}\"\nvalue_label = \"{VALUE_LABEL_ID}\"\n"
    );
    if moon {
        text.push_str(&format!("\n[[texture]]\nname = \"MW12_SKY_MOON\"\nfile = \"{FOLDER}/{MOON_FILE}\"\n"));
    }
    text
}

#[cfg(test)]
mod tests {
    use super::super::plan::{KeySpec, WeatherSpec};
    use super::*;

    fn weather() -> WeatherSpec {
        WeatherSpec {
            max_cover: 0.5,
            gain: 1.5,
            keys: vec![KeySpec { hour: 5.0, dome: "morning", tint: [1.0, 0.5, 0.25] }],
        }
    }

    #[test]
    fn the_profile_text_is_read_by_the_games_parser() {
        let written: Vec<DomeSpec> = DOMES.to_vec();
        let text = profile_text(&[weather()], &written, true);
        let profile = blackbox_modding::content::parse_sky_profile(&text).expect(&text);
        assert_eq!(profile.domes.len(), DOMES.len());
        assert_eq!(profile.domes[1].glow, Some(0.5));
        assert_eq!(profile.weathers[0].gain, 1.5);
        assert_eq!(profile.moon.map(|moon| moon.file), Some("SKY/MW2012/MOON.DDS".to_owned()));
        assert_eq!(profile.exposure, EXPOSURE);
    }

    #[test]
    fn the_replacement_set_is_a_set_of_kind_sky() {
        let text = replace_manifest(true);
        assert!(text.contains("kind = \"sky\"") && text.contains("value = \"mw2012\""));
        assert!(text.contains("[[texture]]"));
        assert!(!replace_manifest(false).contains("[[texture]]"));
    }
}
