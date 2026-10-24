//! The server loop end to end: a server over the toy world on a fake clock, and raw test clients on the in-memory
//! link or on the lossy simulator with the real handshake.

mod client;
mod drops;
mod harness;
mod join;
mod knockout;
mod late;
mod lossy;
mod many;
mod password;
mod quick;
mod results;
mod security;
mod staged;
