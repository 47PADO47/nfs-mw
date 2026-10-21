//! How a movie's picture is cropped and fitted to the window: the film without its baked-in black bars.

use crate::viewer::Fit;

/// The film movies are 1024 x 512 pictures with the black bars of a wide screen baked into them: every frame of every
/// movie but the EA logo has black outside rows 64 up to 448 (measured over the first 500 frames of each movie).
const FILM_ROWS: [f32; 2] = [64.0, 448.0];
const PICTURE_ROWS: f32 = 512.0;
/// The widescreen packages show a movie in a 900 x 480 object; the logo, which has no bars, keeps that shape.
const LOGO_ASPECT: f32 = 900.0 / 480.0;

/// What to show of a movie file's frames and how: the film without its baked-in bars, stretched over the window.
/// With `hd_fmv` every film is framed by the stock 1024 x 512 rule, whatever its size: the bars are at the same
/// fractions of any size, so the same rows show the film (`docs/specs/hd-content-support.md`, section 4).
pub(super) fn framing(file: &str, size: [u32; 2], hd_fmv: bool) -> ([f32; 4], Fit) {
    let whole = [0.0, 0.0, 1.0, 1.0];
    if file.to_ascii_lowercase().starts_with("ealogo") {
        return (whole, Fit::Contain(LOGO_ASPECT));
    }
    if !hd_fmv && size[1] != PICTURE_ROWS as u32 {
        return (whole, Fit::Contain(size[0] as f32 / size[1] as f32));
    }
    ([0.0, FILM_ROWS[0] / PICTURE_ROWS, 1.0, FILM_ROWS[1] / PICTURE_ROWS], Fit::Fill)
}

/// The on-screen rectangle `(x, y, width, height)` of a picture with `fit` in a `w` by `h` window, centred.
pub(super) fn fit_rect(fit: Fit, w: f32, h: f32) -> (f32, f32, f32, f32) {
    let (width, height) = match fit {
        Fit::Fill => (w, h),
        Fit::Contain(aspect) if w / h > aspect => (h * aspect, h),
        Fit::Contain(aspect) => (w, w / aspect),
    };
    ((w - width) * 0.5, (h - height) * 0.5, width, height)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn films_lose_their_baked_bars_and_fill_the_window_but_the_logo_keeps_its_shape() {
        let (view, fit) = framing("psa_english_ntsc.vp6", [1024, 512], false);
        assert_eq!((view, fit), ([0.0, 0.125, 1.0, 0.875], Fit::Fill));
        assert_eq!(
            framing("EALOGO_english_ntsc.vp6", [1024, 512], false),
            ([0.0, 0.0, 1.0, 1.0], Fit::Contain(LOGO_ASPECT))
        );
        let (view, fit) = framing("odd.vp6", [640, 480], false);
        assert_eq!((view, fit), ([0.0, 0.0, 1.0, 1.0], Fit::Contain(640.0 / 480.0)));
    }

    #[test]
    fn hd_movies_are_framed_like_the_stock_film_at_any_size() {
        let stock = framing("psa_english_ntsc.vp6", [1024, 512], true);
        assert_eq!(stock, ([0.0, 0.125, 1.0, 0.875], Fit::Fill));
        assert_eq!(framing("psa_english_ntsc.vp6", [2048, 1024], true), stock);
        assert_eq!(framing("psa_english_ntsc.vp6", [1280, 720], true), stock);
        assert_eq!(framing("odd.vp6", [640, 480], true), stock);
        let logo = ([0.0, 0.0, 1.0, 1.0], Fit::Contain(LOGO_ASPECT));
        assert_eq!(framing("EALOGO_english_ntsc.vp6", [2048, 1024], true), logo);
        assert_eq!(framing("EALOGO_english_ntsc.vp6", [1024, 512], false), logo);
    }

    #[test]
    fn the_hd_film_lands_on_the_same_window_rect_as_the_stock_film() {
        let stock = framing("psa.vp6", [1024, 512], true);
        for size in [[2048, 1024], [1280, 720], [1024, 512]] {
            let (_, fit) = framing("psa.vp6", size, true);
            assert_eq!(fit_rect(fit, 1000.0, 1000.0), fit_rect(stock.1, 1000.0, 1000.0), "{size:?}");
        }
        assert_eq!(fit_rect(stock.1, 1000.0, 1000.0), (0.0, 0.0, 1000.0, 1000.0));
    }

    #[test]
    fn a_stock_size_film_without_the_option_keeps_its_own_aspect_in_the_window() {
        let (_, fit) = framing("odd.vp6", [1280, 720], false);
        let (x, y, width, height) = fit_rect(fit, 1000.0, 1000.0);
        assert!(x.abs() < 1e-3 && (width - 1000.0).abs() < 1e-3, "{x} {width}");
        assert!((height - 562.5).abs() < 1e-2 && (y - 218.75).abs() < 1e-2, "{y} {height}");
    }
}
