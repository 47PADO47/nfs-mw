//! The themeable UI component model of the EA Black Box rewrite. Engine-generic: no rendering, no
//! game data, no game names. A host builds a tree of [`Component`]s, lays it out in FEng units
//! (640 x 480, origin centre), builds a [`DrawList`] and draws it. Which component fills each
//! overridable slot ([`SlotId`]) is chosen by the [`Themes`] registry, which falls back to the Original theme.
//!
//! Model and decisions: `libs/blackbox-ui/README.md`. Design decision: `docs/decisions/0008-ui-themes.md`.
//!
//! ```
//! use blackbox_ui::{
//!     BuildCx, Component, DrawList, DrawNode, LayoutCx, Rect, Skin, SlotId, Theme, ThemeId, Themes,
//! };
//!
//! struct Panel;
//! impl Component for Panel {
//!     fn layout(&mut self, _cx: &LayoutCx<'_>, _bounds: Rect) {}
//!     fn build(&self, out: &mut DrawList) {
//!         out.push(DrawNode { bounds: Rect::SCREEN, colour: [255, 255, 255, 255], z: 0.0 });
//!     }
//! }
//!
//! struct Original;
//! impl Theme for Original {
//!     fn id(&self) -> ThemeId { ThemeId::Original }
//!     fn element(&self, _slot: SlotId, _cx: &BuildCx<'_>) -> Option<Box<dyn Component>> {
//!         Some(Box::new(Panel))
//!     }
//! }
//!
//! let themes = Themes::new(Box::new(Original)).expect("Original is the floor");
//! let skin = Skin::new();
//! let component = themes.resolve(SlotId::new(1), &BuildCx::new(&skin));
//! let mut out = DrawList::new();
//! component.build(&mut out);
//! assert_eq!(out.len(), 1);
//! ```

mod component;
mod context;
mod draw;
mod geom;
mod input;
mod skin;
mod slot;
mod theme;
mod themes;

pub use component::{Component, Empty};
pub use context::{BuildCx, EventCx, LayoutCx};
pub use draw::{DrawList, DrawNode};
pub use geom::Rect;
pub use input::{InputEvent, PadButton, PointerKind};
pub use skin::Skin;
pub use slot::SlotId;
pub use theme::{Theme, ThemeId};
pub use themes::{ThemeError, Themes};
