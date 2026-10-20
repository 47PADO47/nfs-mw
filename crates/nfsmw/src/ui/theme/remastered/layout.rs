//! Where the Remastered cluster sits. Its parts are placed in pixels of the reference picture (603 by 393), and mapped onto
//! the canvas with one scale on both axes, so they keep the picture's shapes.

/// The canvas point of the picture's top-left corner, and the picture's width on the canvas (FEng units, centre origin,
/// y down). The cluster sits in the bottom-right corner, where the original gauges are.
const ORIGIN: (f32, f32) = (176.0, 92.0);
const WIDTH: f32 = 196.0;
const PICTURE_WIDTH: f32 = 603.0;

/// A point of the reference picture on the canvas.
pub(super) fn at(x: f32, y: f32) -> (f32, f32) {
    (ORIGIN.0 + x * scale(), ORIGIN.1 + y * scale())
}

/// A width and height of the reference picture on the canvas.
pub(super) fn size(width: f32, height: f32) -> (f32, f32) {
    (width * scale(), height * scale())
}

/// The rectangle of the picture's box `(left, top, right, bottom)` on the canvas, as a centred `Rect`.
pub(super) fn rect(left: f32, top: f32, right: f32, bottom: f32) -> blackbox_ui::Rect {
    let (x0, y0) = at(left, top);
    let (w, h) = size(right - left, bottom - top);
    blackbox_ui::Rect::new(x0 + w / 2.0, y0 + h / 2.0, w, h)
}

/// Canvas units per picture pixel.
fn scale() -> f32 {
    WIDTH / PICTURE_WIDTH
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_scale_serves_both_axes() {
        let (w, h) = size(100.0, 100.0);
        assert_eq!(w, h);
        assert!((size(PICTURE_WIDTH, 0.0).0 - WIDTH).abs() < 1e-3, "the picture is the cluster's width on the canvas");
        let r = rect(10.0, 20.0, 30.0, 60.0);
        assert!((r.w / r.h - 0.5).abs() < 1e-5);
        assert_eq!((r.min_x(), r.min_y()), at(10.0, 20.0));
    }
}
