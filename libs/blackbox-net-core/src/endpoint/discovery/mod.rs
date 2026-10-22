//! LAN discovery on the net thread (protocol.md section 3, security-ops.md 2.3): the server's responder and the
//! client's scan. The datagrams themselves are [`crate::discovery`].
//!
//! - [`SourcePolicy`]: which source addresses get a reply (`policy.rs`).
//! - [`DiscoveryResponder`]: sans-IO; the server endpoint runs it on its discovery port (`responder.rs`).
//! - [`DiscoveryScan`]: requests to the broadcast address and typed addresses, replies collected on its own thread
//!   (`scan.rs`).

mod policy;
mod responder;
mod scan;

pub use policy::{SourcePolicy, Subnet};
pub use responder::DiscoveryResponder;
pub use scan::{DiscoveryScan, Found, ScanConfig};
