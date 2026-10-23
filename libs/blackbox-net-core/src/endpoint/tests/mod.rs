//! Endpoint tests. Real sockets bind `127.0.0.1` with ports the system picks (no network needed); the lossy tests run
//! the same endpoints over the seeded in-memory simulator. Every wait is bounded ([`support::WAIT`]) and polls, so a
//! slow machine makes a test slower, not flaky.

mod backpressure;
mod discovery;
mod flood;
mod lifecycle;
mod local;
mod loopback;
mod lossy;
mod support;
