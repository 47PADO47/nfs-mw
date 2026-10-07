//! Record layouts per game: the `TrackStreamingSection` index and the
//! `VisibleSectionManager` tables.

mod most_wanted;
mod visible;

pub use most_wanted::{MOST_WANTED, MOST_WANTED_VISIBLE};
pub use visible::{BoundaryLayout, DrivableLayout, InfoLayout, LoadingLayout, VisibleLayout};

/// Field offsets inside one record.
#[derive(Debug, Clone)]
pub struct SectionLayout {
    /// Which game(s) are known to use it.
    pub used_by: &'static str,
    pub len: usize,
    /// `char[8]`.
    pub name: usize,
    /// `i16`.
    pub number: usize,
    pub file_type: usize,
    pub file_offset: usize,
    pub size: usize,
    pub compressed_size: usize,
    pub perm_size: usize,
    pub priority: usize,
    /// `f32` x, y.
    pub center: usize,
    pub radius: usize,
    pub checksum: usize,
}
