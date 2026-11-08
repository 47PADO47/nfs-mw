//! Gauge skins for a car HUD, read from ini text and turned into quads. Engine-generic: pure text and bytes in, numbers and
//! pixels out; no file access, no drawing, no game.
//!
//! - [`General::parse`] reads the settings file that chooses the skin, [`Skin::parse`] the skin's ini.
//! - [`compose`] turns a skin and a frame's [`Inputs`] into the [`Quad`]s to draw, in the order the plugin draws them.
//! - [`Placement`] maps reference pixels (a 1080-line screen, anchored at the right and bottom edge) to canvas units.
//! - [`needle`], [`digits`] and [`mask`] hold the angle, cell and sector rules; [`dds::decode`] reads the pictures.
//!
//! File format: `docs/formats/custom-hud-skin.md`. Behaviour: `docs/specs/custom-hud-gauges.md`.

mod compose;
pub mod dds;
pub mod digits;
mod doc;
mod draw;
mod element;
mod gauge;
mod general;
mod image;
pub mod mask;
pub mod needle;
mod place;
mod readout;
mod skin;
mod value;

pub use compose::{BOOST_LIMIT, compose};
pub use dds::DdsError;
pub use doc::Doc;
pub use draw::{Animation, Inputs, Pictures, Quad, Sector, Turn};
pub use element::{Arrow, Bar, Common, Digital, Gauge, Icon, Masked, Picture};
pub use general::{CarSkins, General, normalise_dir};
pub use image::Image;
pub use place::{Placement, REFERENCE_HEIGHT, Rect};
pub use readout::{bar_fill, blink, shift_light_on};
pub use skin::{PictureUse, Skin};
pub use value::{Colour, parse_float, parse_int};
