//! Where the Remastered cluster sits. Its parts are placed in points of the reference picture (713 by 348 pixels), and
//! mapped onto the canvas, so they keep the picture's shapes.

/// The canvas point of the picture's top-left corner, and the picture's width on the canvas (FEng units, centre origin,
/// y down). The cluster sits in the bottom-right corner, where the original gauges are.
const ORIGIN: (f32, f32) = (176.0, 84.0);
const WIDTH: f32 = 196.0;
const PICTURE_WIDTH: f32 = 713.0;
/// A canvas unit is 2 pixels across and 1.5 down on the 1280 by 720 window, so a picture pixel is taller on the canvas
/// by this factor. Without it the round parts of the picture come out flat.
const ASPECT: f32 = 2.0 / 1.5;

/// A point of the reference picture on the canvas.
pub(super) fn at(x: f32, y: f32) -> (f32, f32) {
    (ORIGIN.0 + x * scale_x(), ORIGIN.1 + y * scale_y())
}

/// A width and height of the reference picture on the canvas.
pub(super) fn size(width: f32, height: f32) -> (f32, f32) {
    (width * scale_x(), height * scale_y())
}

fn scale_x() -> f32 {
    WIDTH / PICTURE_WIDTH
}

fn scale_y() -> f32 {
    scale_x() * ASPECT
}
