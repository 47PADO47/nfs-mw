//! Ids of props that have no spawner.

/// The id of a prop that has no spawner: the high bit, the tile in the next 15 bits and the index in the tile in the low
/// 16. A tile or index too large for that falls back to the arrival counter `next_id`.
pub(super) fn unspawned_id(tile: usize, index: usize, next_id: u32) -> u32 {
    match (u16::try_from(tile).ok().filter(|&tile| tile < 0x8000), u16::try_from(index).ok()) {
        (Some(tile), Some(index)) => 0x8000_0000 | (u32::from(tile) << 16) | u32::from(index),
        _ => next_id | 0x8000_0000,
    }
}

#[cfg(test)]
mod tests {
    use super::unspawned_id;

    #[test]
    fn a_prop_without_a_spawner_is_numbered_by_its_tile_and_place_and_not_by_arrival() {
        assert_eq!(unspawned_id(3, 7, 1), unspawned_id(3, 7, 999_999), "the counter does not matter");
        assert_ne!(unspawned_id(3, 7, 1), unspawned_id(7, 3, 1));
        assert_eq!(unspawned_id(0, 0, 1) & 0x8000_0000, 0x8000_0000, "the high bit keeps them apart from spawner ids");
        assert_eq!(unspawned_id(0x7FFF, 0xFFFF, 1), 0xFFFF_FFFF);
        assert_eq!(unspawned_id(0x8000, 0, 5), 5 | 0x8000_0000, "a tile too large falls back to the counter");
        assert_eq!(unspawned_id(0, 0x1_0000, 5), 5 | 0x8000_0000, "so does an index too large");
    }
}
