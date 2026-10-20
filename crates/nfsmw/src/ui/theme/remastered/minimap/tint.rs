//! `Tinted`: a part drawn with its colours multiplied by a tint, so the game's own picture (the map tiles, the arrow) takes
//! the theme's colour.

use blackbox_ui::{Component, DrawList, LayoutCx, Rect};

pub(super) struct Tinted {
    inner: Box<dyn Component>,
    tint: [u8; 4],
    mode: Mode,
}

#[derive(Clone, Copy)]
enum Mode {
    /// Each channel is scaled by the tint's.
    Multiply,
    /// The colour is the tint's, and the opacity the part's own times the tint's.
    Replace,
}

impl Tinted {
    /// `inner` with each colour multiplied by `tint`.
    pub(super) fn new(inner: impl Component + 'static, tint: [u8; 4]) -> Self {
        Self { inner: Box::new(inner), tint, mode: Mode::Multiply }
    }

    /// `inner` in the colour `colour`, whatever colour the game gave it.
    pub(super) fn recoloured(inner: impl Component + 'static, colour: [u8; 4]) -> Self {
        Self { inner: Box::new(inner), tint: colour, mode: Mode::Replace }
    }
}

impl Component for Tinted {
    /// The part is laid out where it is made.
    fn layout(&mut self, _cx: &LayoutCx<'_>, _bounds: Rect) {}

    fn build(&self, out: &mut DrawList) {
        let mut inner = DrawList::new();
        self.inner.build(&mut inner);
        for node in inner.iter() {
            let mut node = node.clone();
            node.colour = self.mode.apply(node.colour, self.tint);
            node.world_colour = self.mode.apply(node.world_colour, self.tint);
            // The links are positions in the inner list, which `out` does not share.
            node.parent = None;
            out.push(node);
        }
    }
}

impl Mode {
    fn apply(self, colour: [u8; 4], tint: [u8; 4]) -> [u8; 4] {
        match self {
            Mode::Multiply => multiplied(colour, tint),
            Mode::Replace => [tint[0], tint[1], tint[2], multiplied(colour, tint)[3]],
        }
    }
}

fn multiplied(colour: [u8; 4], tint: [u8; 4]) -> [u8; 4] {
    std::array::from_fn(|i| (u16::from(colour[i]) * u16::from(tint[i]) / 255) as u8)
}

#[cfg(test)]
mod tests {
    use super::{Mode, multiplied};

    #[test]
    fn a_tint_scales_each_channel() {
        assert_eq!(multiplied([200, 100, 50, 255], [255, 128, 0, 128]), [200, 50, 0, 128]);
        assert_eq!(multiplied([255; 4], [255; 4]), [255; 4]);
    }

    #[test]
    fn a_recolour_keeps_only_the_opacity() {
        assert_eq!(Mode::Replace.apply([10, 200, 30, 128], [255, 200, 80, 255]), [255, 200, 80, 128]);
    }
}
