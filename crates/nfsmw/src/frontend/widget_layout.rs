//! The objects and the layout of the widget menus: the objects of one row, and where the rows go.

use blackbox_feng::ObjectRef;
use glam::Vec2;

/// The objects of one row.
pub(super) struct RowObjects {
    pub(super) name: ObjectRef,
    pub(super) guid: u32,
    pub(super) data: Option<ObjectRef>,
    pub(super) left: Option<ObjectRef>,
    pub(super) right: Option<ObjectRef>,
    pub(super) base: Option<ObjectRef>,
    pub(super) fill: Option<ObjectRef>,
}

/// Where rows go, from the package's two master objects (the title column and the data column).
pub(super) struct Layout {
    /// Top-left of the title column, and its size.
    pub(super) title: Vec2,
    pub(super) title_size: Vec2,
    /// Top-left of the data column, and its size.
    pub(super) data: Vec2,
    pub(super) data_size: Vec2,
    /// The masters the two columns are read from; their boxes are where the rows are drawn.
    pub(super) title_object: ObjectRef,
    pub(super) data_object: ObjectRef,
}
