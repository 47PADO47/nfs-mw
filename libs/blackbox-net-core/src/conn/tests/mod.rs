//! Tests of the connection: codec units, connection-level refusals and timers, the two-connection loss simulation, and a
//! seeded fuzz pass. The harness in `harness.rs` moves datagrams between two connections with loss, duplication and delay.

mod codec;
mod errors;
mod fuzz;
mod harness;
mod loss;
