//! Hand-off between the net thread and the game thread (integration.md section 1), behind the `handoff` feature.
//!
//! - [`ring`]: a bounded queue for events, messages and outgoing input. One producer, one consumer. When it is full the
//!   **oldest** entry is dropped and counted, so the producer never waits. Built on `crossbeam-queue`'s lock-free
//!   `ArrayQueue`.
//! - [`latest`]: a latest-value cell for snapshots, a triple buffer (`triple_buffer` crate). The writer publishes
//!   whole values, the reader always gets the newest, neither waits.
//!
//! The game thread never waits on a socket, a lock or a file: every call here is wait-free or lock-free.

mod latest;
mod ring;
#[cfg(test)]
mod tests;

pub use latest::{LatestReader, LatestWriter, latest};
pub use ring::{RingReceiver, RingSender, ring};
