//! The elements of a skin and how each section of the ini fills them. Spec: `docs/formats/custom-hud-skin.md` section 4.

use crate::doc::Doc;
use crate::value::Colour;

/// The keys every element has. An element whose `Enabled` is 0 is not read at all.
#[derive(Debug, Clone, PartialEq)]
pub struct Common {
    /// The element's side, in reference pixels.
    pub size: f32,
    /// The distance of the element's right and bottom edge from the screen's, in reference pixels.
    pub x: f32,
    pub y: f32,
    pub colour: Colour,
}

impl Common {
    /// The common keys of `section`, or `None` when the section is absent or not enabled.
    pub fn read(doc: &Doc, section: &str) -> Option<Self> {
        if !doc.flag(section, "Enabled", false) {
            return None;
        }
        Some(Self {
            size: doc.float(section, "Size", 0.0),
            x: doc.float(section, "PositionX", 0.0),
            y: doc.float(section, "PositionY", 0.0),
            colour: doc.colour(section, "Color"),
        })
    }
}

/// A picture file with its blend and tint. An empty `file` means the picture is not drawn.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Picture {
    pub file: String,
    /// The blend mode key was non-zero: additive instead of normal alpha blending.
    pub additive: bool,
    pub colour: Colour,
}

impl Picture {
    fn read(doc: &Doc, section: &str, file_key: &str, blend_key: &str, colour_key: &str) -> Self {
        Self {
            file: doc.text(section, file_key),
            additive: doc.int(section, blend_key, 0) != 0,
            colour: doc.colour(section, colour_key),
        }
    }

    /// Whether there is a picture to draw.
    pub fn is_set(&self) -> bool {
        !self.file.is_empty()
    }
}

/// A picture drawn through a sector mask: the arc's filled picture or the red zone's numbers.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Masked {
    pub picture: Picture,
    /// The tint near the red line, and how near (thousands of rpm); only the arc has them.
    pub max_colour: Colour,
    pub max_threshold: f32,
    /// The mask's value outside and inside the sector; only the alpha byte counts.
    pub outside: Colour,
    pub inside: Colour,
    /// The mask bitmap's side in pixels. This port builds the mask at the picture's own size and keeps the key for the spec.
    pub size: f32,
    /// Degrees the sector starts before its start (the arc), and ends after its end (the numbers).
    pub min_offset: f32,
    pub max_offset: f32,
}

/// The needle.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Arrow {
    pub picture: Picture,
    /// The tint while the game's "perfect zone" flag is set; unset keeps `picture.colour`.
    pub perfect_colour: Colour,
    /// 1 when the key is absent.
    pub scale: f32,
    /// Where the needle turns: 0 at its right end, 1 at its left end; 0.22 when the key is absent.
    pub centre_offset: f32,
}

/// A dial, with its needle and masked pictures.
#[derive(Debug, Clone, PartialEq)]
pub struct Gauge {
    pub common: Common,
    /// The value at the end of the scale.
    pub value: f32,
    pub min_angle: f32,
    pub max_angle: f32,
    pub can_be_negative: bool,
    pub hide_on_max: bool,
    pub background: Picture,
    pub numbers: Picture,
    pub numbers_max_colour: Colour,
    pub numbers_max_threshold: f32,
    pub numbers_masked: Masked,
    pub arrow: Arrow,
    pub arrow_masked: Masked,
}

impl Gauge {
    pub fn read(doc: &Doc, section: &str) -> Option<Self> {
        let common = Common::read(doc, section)?;
        Some(Self {
            common,
            value: doc.float(section, "Value", 0.0),
            min_angle: doc.float(section, "ArrowMinAngle", 0.0),
            max_angle: doc.float(section, "ArrowMaxAngle", 0.0),
            can_be_negative: doc.flag(section, "CanBeNegative", true),
            hide_on_max: doc.int(section, "HideOnMaxValue", 0) == 1,
            background: Picture::read(
                doc,
                section,
                "BackgroundTexture",
                "BackgroundTextureBlendMode",
                "BackgroundColor",
            ),
            numbers: Picture::read(doc, section, "NumbersTexture", "NumbersTextureBlendMode", "NumbersColor"),
            numbers_max_colour: doc.colour(section, "NumbersMaxColor"),
            numbers_max_threshold: doc.float(section, "NumbersMaxThreshold", -1.0),
            numbers_masked: Masked {
                picture: Picture::read(
                    doc,
                    section,
                    "NumbersMaskedTexture",
                    "NumbersMaskedTextureBlendMode",
                    "NumbersMaskedColor",
                ),
                outside: doc.colour(section, "NumbersMaskedColor1"),
                inside: doc.colour(section, "NumbersMaskedColor2"),
                size: doc.float(section, "NumbersMaskedSize", 128.0),
                max_offset: doc.float(section, "NumbersMaskedMaxOffset", 0.0),
                ..Masked::default()
            },
            arrow: Arrow {
                picture: Picture::read(doc, section, "ArrowTexture", "ArrowTextureBlendMode", "ArrowColor"),
                perfect_colour: doc.colour(section, "ArrowPerfectZoneColor"),
                scale: doc.float(section, "ArrowScale", 1.0),
                centre_offset: doc.float(section, "ArrowCenterOffset", 0.22),
            },
            arrow_masked: Masked {
                picture: Picture::read(
                    doc,
                    section,
                    "ArrowMaskedTexture",
                    "ArrowMaskedTextureBlendMode",
                    "ArrowMaskedColor",
                ),
                max_colour: doc.colour(section, "ArrowMaskedMaxColor"),
                max_threshold: doc.float(section, "ArrowMaskedMaxThreshold", 0.0),
                outside: doc.colour(section, "ArrowMaskedColor1"),
                inside: doc.colour(section, "ArrowMaskedColor2"),
                size: doc.float(section, "ArrowMaskedSize", 128.0),
                min_offset: doc.float(section, "ArrowMaskedMinOffset", 0.0),
                max_offset: 0.0,
            },
        })
    }
}

/// A row of digits: the speed, the rev counter or the gear.
#[derive(Debug, Clone, PartialEq)]
pub struct Digital {
    pub common: Common,
    pub picture: Picture,
    /// The colour of a ghost "8" drawn behind each digit; unset draws none.
    pub ghost: Colour,
    /// Added to `common.size` to get the step from one digit to the next.
    pub distance: f32,
    /// 0, 1 or 2 (`docs/specs/custom-hud-gauges.md` section 7).
    pub mode: i32,
    /// Other pictures for the tens and the hundreds digit of the speed; empty keeps `picture`.
    pub tens_file: String,
    pub hundreds_file: String,
}

impl Digital {
    pub fn read(doc: &Doc, section: &str) -> Option<Self> {
        let common = Common::read(doc, section)?;
        Some(Self {
            picture: Picture::read(doc, section, "DigitsTexture", "DigitsTextureBlendMode", "Color"),
            common,
            ghost: doc.colour(section, "BackgroundColor"),
            distance: doc.float(section, "Distance", 0.0),
            mode: doc.int(section, "DigitsMode", 0) as i32,
            tens_file: doc.text(section, "DigitsTexture1"),
            hundreds_file: doc.text(section, "DigitsTexture2"),
        })
    }
}

/// A bar that fills from one side.
#[derive(Debug, Clone, PartialEq)]
pub struct Bar {
    pub common: Common,
    /// The value at which the bar is full.
    pub value: f32,
    /// 0 fills from the left, 1 from the bottom, 2 from the right.
    pub direction: i32,
    pub filled: Picture,
    pub background: Picture,
}

impl Bar {
    pub fn read(doc: &Doc, section: &str) -> Option<Self> {
        let common = Common::read(doc, section)?;
        Some(Self {
            common,
            value: doc.float(section, "Value", 0.0),
            direction: doc.int(section, "Direction", 0) as i32,
            filled: Picture::read(doc, section, "FilledTexture", "FilledTextureBlendMode", "FilledColor"),
            background: Picture::read(
                doc,
                section,
                "BackgroundTexture",
                "BackgroundTextureBlendMode",
                "BackgroundColor",
            ),
        })
    }
}

/// A single picture element: the background, the units or the shift light.
#[derive(Debug, Clone, PartialEq)]
pub struct Icon {
    pub common: Common,
    pub picture: Picture,
    /// The shift light's blink mode: 0 steady, 1 blink, 2 pulse.
    pub blink: i32,
}

impl Icon {
    /// An icon whose picture and blend keys are named `file_key` and `blend_key`.
    pub fn read(doc: &Doc, section: &str, file_key: &str, blend_key: &str) -> Option<Self> {
        let common = Common::read(doc, section)?;
        Some(Self {
            picture: Picture::read(doc, section, file_key, blend_key, "Color"),
            common,
            blink: doc.int(section, "BlinkMode", 0) as i32,
        })
    }
}
