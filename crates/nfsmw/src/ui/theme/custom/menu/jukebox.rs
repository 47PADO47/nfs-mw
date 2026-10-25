//! The EA Trax playlist (`Jukebox`): the songs as the rows of a settings list, `Artist - Title` with the song's mode
//! right-aligned in the value column, and the order of play (and whether the list is being re-ordered) beside the
//! title rather than on the description line, since it is about the whole list, not the selected row.

use super::Parts;
use crate::ui::theme::MenuList;

impl Parts<'_, '_> {
    /// The playlist, with the selection plate at `plate_y`.
    pub(super) fn jukebox(&mut self, list: &MenuList, plate_y: Option<f32>) {
        let playlist = MenuList { message: String::new(), ..list.clone() };
        self.list(&playlist, plate_y);
        self.subtitle(&list.message);
    }
}
