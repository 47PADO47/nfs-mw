//! Handles: 64-bit ids for objects the host owns (vehicles, blips, commands, actions, abilities, menus, worlds).
//!
//! Layout: the low 32 bits index a slot, the high 32 bits are the slot's generation. A generation starts at 1, so a
//! live handle is never `BB_NULL_HANDLE`. When an object goes away its slot's generation rises, and an old handle to
//! it answers `BB_ERR_STALE` instead of naming whatever reuses the slot.

/// The handle that names nothing. Never a live handle.
pub const BB_NULL_HANDLE: u64 = 0;

/// The slot index of a handle (its low 32 bits).
pub const fn handle_index(handle: u64) -> u32 {
    handle as u32
}

/// The generation of a handle (its high 32 bits).
pub const fn handle_generation(handle: u64) -> u32 {
    (handle >> 32) as u32
}

/// Pack a slot index and a generation into a handle.
pub const fn make_handle(index: u32, generation: u32) -> u64 {
    ((generation as u64) << 32) | index as u64
}
