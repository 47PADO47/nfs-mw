//! The client side of a multiplayer session (integration.md section 2, `docs/specs/multiplayer/client-session.md`):
//! everything between the network endpoint and the game, as one driver the game calls once a frame.
//!
//! [`ClientSession`] is generic over a [`ClientLink`] (the net thread's `ClientEndpoint`, or a [`MockLink`] whose
//! [`MockPeer`] plays the server in tests) and a [`crate::predict::PredictedCar`] (the game's vehicle, or `ToyCar`).
//! It reads no clock and never blocks: [`ClientSession::update`] takes the caller's time.
//!
//! - Admission: `Auth` and `Join` on connect, the room password (SPAKE2 bound to the handshake hash), the schema check
//!   of `JoinAccepted`, and the resume token: after a silent drop the session reconnects (through a [`Connector`] or
//!   [`ClientSession::resume_with`]) and presents the token for 30 s.
//! - The lobby mirror ([`LobbyView`]): roster, settings, phase, host, countdown, `LoadWorld`, start tick, chat, notices,
//!   votes, roles, the kick that ended the session, and the late-join session data from the bulk channel.
//! - Snapshots: parts reassembled in any order (`parts.rs`), decoded against the client's views of the last 64 ticks
//!   (`views.rs`), remote entities to the interpolator, the own block to the predictor and reconciler, the global block
//!   to [`GlobalView`]; a lost baseline is reported with `BaselineLost`.
//! - The clock: PING on the schedule, PONG into `NetClock`; a clock step voids the prediction.
//! - Input: [`ClientSession::due_ticks`] and [`ClientSession::submit_input`] (or [`ClientSession::drive`]) step the
//!   owner's car and send INPUT frames from the input history; neutral before the race start; none from a spectator.
//! - Events ([`SessionEvent`]): admission, lobby, chat, countdown, load, gameplay events after the `seq` and target
//!   tick gate, results, snaps, the end of the session ([`EndReason`]).
//! - Spectating: no input, a follow camera ([`ClientSession::follow`], [`ClientSession::follow_target`]).
//!
//! **Files:** `session.rs` (the struct and its getters), `update.rs` (the frame, the link, reconnect), `admission.rs`,
//! `dispatch.rs` (reliable messages), `snapshot.rs`, `commands.rs`, `own_car.rs`, `remote.rs`, `views.rs`, `parts.rs`,
//! `lobby.rs`, `events.rs`, `status.rs`, `config.rs`, `link.rs`, `mock.rs`. Tests in `tests/` play a scripted server
//! over the mock link.

mod admission;
mod commands;
mod config;
mod dispatch;
mod events;
mod link;
mod lobby;
mod mock;
mod own_car;
mod parts;
mod remote;
mod session;
mod snapshot;
mod status;
mod update;
mod views;

#[cfg(test)]
mod tests;

pub use config::SessionConfig;
pub use events::{EventGate, SessionEvent, event_head};
pub use link::ClientLink;
pub use lobby::{ChatLine, LobbyView};
pub use mock::{MockLink, MockPeer, Sent, mock_link};
pub use session::{ClientSession, Connector, GlobalView, SessionCounters};
pub use status::{EndReason, Status};
