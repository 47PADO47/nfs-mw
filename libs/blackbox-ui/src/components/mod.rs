//! The base components: generic building blocks a theme composes into its UI. Each one lays out inside the `Rect`
//! it is given, caches what `build` needs in `layout`, and builds `UiNode`s from values the `Skin` supplies, so a
//! different skin gives a different look with no new code.
//!
//! Draw order: a node keeps the `z` of the component that built it (larger is farther). Among equal `z`, the
//! later node draws on top, so a component builds its parts in back-to-front order.

mod chevron;
mod choice;
mod gauge;
mod image;
mod outline;
mod panel;
mod pill_bar;
mod polyline;
mod progress;
mod ring;
mod rounded;
mod row;
mod slant;
mod stack;
mod switch;
mod table;
mod text;
mod trapezoid;

pub use chevron::{Chevron, Facing, bounce};
pub use choice::{Choice, ChoiceStyle};
pub use gauge::Gauge;
pub use image::Image;
pub use outline::Outline;
pub use panel::Panel;
pub use pill_bar::{PillBar, PillStyle};
pub use polyline::{Clip, LineCap, LineJoin, Polyline, Polylines};
pub use progress::LinearProgress;
pub use ring::Ring;
pub use rounded::Rounded;
pub use row::Row;
pub use slant::Slant;
pub use stack::{Stack, StackAxis};
pub use switch::Switch;
pub use table::Table;
pub use text::{Text, justify};
pub use trapezoid::Trapezoid;

use glam::{Mat4, Vec3};

use crate::tree::{NodeKind, UiNode};

/// The depth a component draws at unless its builder sets one. FEng-style producers keep nodes with `z > 0`.
pub const DEFAULT_Z: f32 = 1.0;

/// The colour a component uses when the skin has no value for its key: opaque white.
pub(crate) const FALLBACK_COLOUR: [u8; 4] = [255, 255, 255, 255];

/// The whole texture, as a UV rectangle `[u0, v0, u1, v1]`.
pub(crate) const FULL_UV: [f32; 4] = [0.0, 0.0, 1.0, 1.0];

/// A mask pivot and rotation that leave the mask unrotated: the centre of the texture, zero degrees.
pub(crate) const UNROTATED: [f32; 3] = [0.5, 0.5, 0.0];

/// A node with the given world transform, colour and depth. Its `world_colour` equals its colour, because a base
/// component is never nested in a tinted parent. `position` is the transform's origin, and `size` its scale.
pub(crate) fn node_at(world: Mat4, z: f32, colour: [u8; 4], kind: NodeKind) -> UiNode {
    UiNode {
        kind,
        colour,
        world_colour: colour,
        world,
        position: world.w_axis.truncate(),
        size: Vec3::new(world.x_axis.length(), world.y_axis.length(), 1.0),
        z,
        visible: true,
        ..UiNode::default()
    }
}

#[cfg(test)]
pub(crate) mod testing {
    use crate::component::Component;
    use crate::context::LayoutCx;
    use crate::draw::DrawList;
    use crate::geom::Rect;
    use crate::skin::Skin;

    /// Lays `c` out in `bounds` with the given skin.
    pub(crate) fn lay_out(c: &mut dyn Component, skin: &Skin, bounds: Rect) {
        c.layout(&LayoutCx::new(skin, 0.0), bounds);
    }

    /// Builds `c` into a fresh draw list.
    pub(crate) fn drawn(c: &dyn Component) -> DrawList {
        let mut out = DrawList::new();
        c.build(&mut out);
        out
    }
}
