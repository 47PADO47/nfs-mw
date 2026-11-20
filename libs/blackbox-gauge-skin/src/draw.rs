//! What a frame of a skin asks the renderer to draw: a list of [`Quad`]s in back-to-front order, in the presenter's canvas
//! units. The renderer turns each into its own primitive; nothing here draws.

use crate::place::Rect;
use crate::value::Colour;

/// A picture drawn over a rectangle.
#[derive(Debug, Clone, PartialEq)]
pub struct Quad {
    /// The picture's file name as the skin writes it.
    pub picture: String,
    pub additive: bool,
    /// Where the picture lands before any turn: centre and size, canvas units.
    pub rect: Rect,
    /// The part of the picture shown, `[u0, v0, u1, v1]`.
    pub uv: [f32; 4],
    pub colour: Colour,
    pub turn: Option<Turn>,
    /// Draw the picture through a sector mask (the filled arc, the red-zone numbers).
    pub sector: Option<Sector>,
}

/// A turn of the whole quad about a point of the canvas, clockwise on the screen.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Turn {
    pub radians: f32,
    pub about: (f32, f32),
}

/// A sector mask: the picture's alpha is scaled by `inside` (0 to 255) in the clockwise sweep from `start` to `end`
/// (degrees, the needle's angles) and by `outside` elsewhere.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sector {
    pub start: f32,
    pub end: f32,
    pub inside: u8,
    pub outside: u8,
    /// The sector changes rarely (the red zone's numbers), so the renderer may build its masked picture at full detail; a
    /// moving arc is built smaller.
    pub fine: bool,
}

/// The values a frame reads, in the units the skin expects.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Inputs {
    /// Engine speed, rpm.
    pub rpm: f32,
    /// The red line, rpm.
    pub red_line: f32,
    /// The speed in the shown unit (not cut to whole numbers).
    pub speed: f32,
    /// The speed is in miles per hour.
    pub mph: bool,
    /// -1 reverse, 0 neutral, 1 and up forward.
    pub gear: i32,
    /// The nitrous left, 0 to 1, or `None` for a car without a nitrous system.
    pub nitrous: Option<f32>,
    /// The speed breaker meter, 0 to 1.
    pub speedbreaker: f32,
    /// The boost, or `None` for a car without forced induction.
    pub boost: Option<f32>,
    /// The tachometer's "perfect zone" flag.
    pub perfect_zone: bool,
    /// Seconds since the last frame.
    pub dt: f32,
}

impl Default for Inputs {
    fn default() -> Self {
        Self {
            rpm: 0.0,
            red_line: 7500.0,
            speed: 0.0,
            mph: false,
            gear: 0,
            nitrous: None,
            speedbreaker: 0.0,
            boost: None,
            perfect_zone: false,
            dt: 0.0,
        }
    }
}

/// What the renderer knows about the pictures: their size in pixels.
pub trait Pictures {
    /// The size of the picture `file` of this skin, or `None` when it did not load.
    fn size(&self, file: &str) -> Option<(u32, u32)>;
}

/// The state that carries from one frame to the next: the fades of the gauges that hide at their maximum, and the shift
/// light's blink timer.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Animation {
    pub(crate) fades: [f32; 5],
    pub(crate) shift_timer: f32,
}
