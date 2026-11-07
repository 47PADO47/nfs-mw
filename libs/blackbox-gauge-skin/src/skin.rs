//! A whole skin: every element its ini enables, and the list of pictures it needs.

use crate::doc::Doc;
use crate::element::{Bar, Digital, Gauge, Icon, Picture};

/// One picture file a skin names and how it is blended. A file used with two blend modes appears twice.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PictureUse {
    /// The name as the skin writes it, relative to the skin's folder.
    pub file: String,
    pub additive: bool,
}

/// The elements of one skin ini. An element the ini does not enable is `None`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Skin {
    pub background: Option<Icon>,
    pub tachometer: Option<Gauge>,
    pub tachometer_digits: Option<Digital>,
    pub tachometer_bar: Option<Bar>,
    pub speedometer: Option<Gauge>,
    pub speed_digits: Option<Digital>,
    pub nitrous: Option<Gauge>,
    pub nitrous_bar: Option<Bar>,
    pub boost: Option<Gauge>,
    pub speedbreaker: Option<Gauge>,
    pub speedbreaker_bar: Option<Bar>,
    pub units: Option<Icon>,
    pub gear: Option<Digital>,
    pub shift_light: Option<Icon>,
}

impl Skin {
    /// Reads the text of a skin ini (spec: `docs/formats/custom-hud-skin.md` section 4).
    pub fn parse(text: &str) -> Self {
        let doc = Doc::parse(text);
        Self {
            background: Icon::read(&doc, "BACKGROUND", "Texture", "TextureBlendMode"),
            tachometer: Gauge::read(&doc, "TACHOMETER_GAUGE"),
            tachometer_digits: Digital::read(&doc, "TACHOMETER_DIGITAL"),
            tachometer_bar: Bar::read(&doc, "TACHOMETER_LINIAR"),
            speedometer: Gauge::read(&doc, "SPEEDOMETER_GAUGE"),
            speed_digits: Digital::read(&doc, "SPEEDOMETER_DIGITAL"),
            nitrous: Gauge::read(&doc, "NOS_GAUGE"),
            nitrous_bar: Bar::read(&doc, "NOS_LINIAR"),
            boost: Gauge::read(&doc, "BOOST_GAUGE"),
            speedbreaker: Gauge::read(&doc, "SPEEDBREAK_GAUGE"),
            speedbreaker_bar: Bar::read(&doc, "SPEEDBREAK_LINIAR"),
            units: Icon::read(&doc, "UNITS", "UnitsTexture", "UnitsTextureBlendMode"),
            gear: Digital::read(&doc, "GEAR"),
            shift_light: Icon::read(&doc, "SHIFT_ICON", "IconTexture", "IconTextureBlendMode"),
        }
    }

    /// Whether any element is enabled.
    pub fn is_empty(&self) -> bool {
        *self == Skin::default()
    }

    /// Every picture the skin draws, once each: the loader reads these and nothing else.
    pub fn pictures(&self) -> Vec<PictureUse> {
        let mut uses = Vec::new();
        let mut add = |picture: &Picture| {
            if !picture.is_set() {
                return;
            }
            let entry = PictureUse { file: picture.file.clone(), additive: picture.additive };
            if !uses.contains(&entry) {
                uses.push(entry);
            }
        };
        for gauge in
            [&self.tachometer, &self.speedometer, &self.nitrous, &self.boost, &self.speedbreaker].into_iter().flatten()
        {
            add(&gauge.background);
            add(&gauge.numbers);
            add(&gauge.numbers_masked.picture);
            add(&gauge.arrow.picture);
            add(&gauge.arrow_masked.picture);
        }
        for digital in [&self.tachometer_digits, &self.speed_digits, &self.gear].into_iter().flatten() {
            add(&digital.picture);
            for file in [&digital.tens_file, &digital.hundreds_file] {
                add(&Picture {
                    file: file.clone(),
                    additive: digital.picture.additive,
                    colour: digital.picture.colour,
                });
            }
        }
        for bar in [&self.tachometer_bar, &self.nitrous_bar, &self.speedbreaker_bar].into_iter().flatten() {
            add(&bar.filled);
            add(&bar.background);
        }
        for icon in [&self.background, &self.units, &self.shift_light].into_iter().flatten() {
            add(&icon.picture);
        }
        uses
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::Colour;

    /// A skin written for these tests: a tachometer with a needle, a speed readout, a gear and a units picture.
    const SAMPLE: &str = "#a test skin\n[TACHOMETER_GAUGE]\nEnabled = 1\nSize = 330\nPositionX = 30\nPositionY = 60\nValue = 10\nArrowTexture = needle.dds\nArrowColor = 0xFFFFFFFF\nArrowPerfectZoneColor = 0xFFFF0000\nArrowMinAngle = -30\nArrowMaxAngle = 210\nArrowTextureBlendMode = 1\nBackgroundTexture = dial.dds\nBackgroundColor = 0xFF8ec8ff\nNumbersTexture = numbers.dds\nNumbersMaskedTexture = numbers.dds\nNumbersMaskedColor = 0xFFFF0000\nNumbersMaskedColor2 = 0xFFFFFFFF\nNumbersMaskedSize = 512\nNumbersMaskedMaxOffset = 15\nNumbersMaxThreshold = 0.5\n\n[GEAR]\nEnabled = 1\nSize = 130\nPositionX = 130\nPositionY = 110\nColor = 0xFFFFFFFF\nDigitsTexture = gear.dds\nDigitsTextureBlendMode= 1\n\n[SPEEDOMETER_DIGITAL]\nEnabled = 1\nSize = 65\nPositionX = 130\nPositionY = 83\nColor = 0xAA0c1a30\nDigitsMode = 2\nDistance = -35\nDigitsTexture = digits.dds\nDigitsTexture1 = tens.dds\n\n[UNITS]\nEnabled = 1\nSize = 64\nPositionX = 128\nPositionY = 45\nUnitsTexture = units.dds\n\n[NOS_GAUGE]\nEnabled = 0\nSize = 90\n";

    #[test]
    fn the_enabled_elements_are_read_and_the_others_are_none() {
        let skin = Skin::parse(SAMPLE);
        let tach = skin.tachometer.as_ref().expect("the tachometer is enabled");
        assert_eq!((tach.common.size, tach.common.x, tach.common.y), (330.0, 30.0, 60.0));
        assert_eq!((tach.value, tach.min_angle, tach.max_angle), (10.0, -30.0, 210.0));
        assert!(tach.can_be_negative, "CanBeNegative defaults to on");
        assert!(!tach.hide_on_max);
        assert_eq!(tach.arrow.picture.file, "needle.dds");
        assert!(tach.arrow.picture.additive);
        assert_eq!(tach.arrow.perfect_colour, Colour(0xFFFF_0000));
        assert_eq!(tach.background.colour, Colour(0xFF8E_C8FF));
        assert_eq!(tach.numbers_max_threshold, 0.5);
        assert_eq!(tach.numbers_masked.size, 512.0);
        assert_eq!((tach.arrow.scale, tach.arrow.centre_offset), (1.0, 0.22), "absent keys read their defaults");
        assert_eq!(tach.numbers_masked.max_offset, 15.0);
        assert_eq!(tach.numbers_masked.inside, Colour(0xFFFF_FFFF));
        assert_eq!(tach.numbers_masked.outside, Colour::NONE);
        assert!(skin.nitrous.is_none(), "Enabled = 0 reads nothing");
        assert!(skin.boost.is_none());
        assert!(skin.tachometer_digits.is_none());
    }

    #[test]
    fn digits_gear_and_units_read_their_own_keys() {
        let skin = Skin::parse(SAMPLE);
        let speed = skin.speed_digits.as_ref().expect("the speed readout is enabled");
        assert_eq!((speed.mode, speed.distance, speed.common.size), (2, -35.0, 65.0));
        assert_eq!(speed.picture.colour, Colour(0xAA0C_1A30));
        assert_eq!(speed.tens_file, "tens.dds");
        assert_eq!(speed.hundreds_file, "");
        let gear = skin.gear.as_ref().expect("the gear is enabled");
        assert!(gear.picture.additive);
        assert_eq!(gear.picture.file, "gear.dds");
        let units = skin.units.as_ref().expect("the units are enabled");
        assert_eq!(units.picture.file, "units.dds");
    }

    #[test]
    fn the_picture_list_has_each_file_and_blend_once() {
        let pictures = Skin::parse(SAMPLE).pictures();
        let has = |file: &str, additive: bool| pictures.contains(&PictureUse { file: file.to_owned(), additive });
        assert!(has("needle.dds", true));
        assert!(has("dial.dds", false));
        assert!(has("numbers.dds", false));
        assert!(has("gear.dds", true));
        assert!(has("digits.dds", false));
        assert!(has("tens.dds", false));
        assert!(has("units.dds", false));
        assert_eq!(pictures.iter().filter(|p| p.file == "numbers.dds").count(), 1, "the numbers file is shared");
        assert!(!pictures.iter().any(|p| p.file.is_empty()));
    }

    #[test]
    fn an_empty_or_disabled_ini_is_an_empty_skin() {
        assert!(Skin::parse("").is_empty());
        assert!(Skin::parse("[TACHOMETER_GAUGE]\nEnabled = 0\nSize = 3\n").is_empty());
        assert!(!Skin::parse(SAMPLE).is_empty());
    }
}
