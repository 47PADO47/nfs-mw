/// How a [`Fullscreen`] picture meets the window.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Fit {
    /// Keep this width over height; black bands fill what is left of the window.
    Contain(f32),
    /// Stretch over the whole window, whatever its shape.
    Fill,
}

/// A picture a scene wants over the whole window (a movie frame).
pub struct Fullscreen {
    pub size: [u32; 2],
    /// The part of the picture to show, as `[u0, v0, u1, v1]` fractions (the rest is baked-in black bars).
    pub view: [f32; 4],
    pub fit: Fit,
    /// A new frame to upload (RGBA8, `size`); `None` keeps the one on screen.
    pub rgba: Option<Vec<u8>>,
}
