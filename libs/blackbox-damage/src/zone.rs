/// The damage levels of one impact zone, each from 0 to [`crate::MAX_LEVEL`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Zone {
    /// Deformation of the panels in the zone.
    pub panel: u8,
    /// Scrapes through the paint.
    pub paint: u8,
    /// Cracks in the glazing; only zones that have glass ever raise it.
    pub glass: u8,
}
