//! The session and per-peer state machines (ADR 0009 section 6). Generic: no game modes, no messages. The server drives
//! one [`Session`]-to-be; the game maps phases to screens and messages.
//!
//! CONTRACT (to implement):
//!
//! - `pub enum Phase { Lobby, Loading, WaitLoads, Intro, Countdown { start_tick: u32 }, Running, Results }`.
//! - `pub enum PeerState { Lobby, Loading, Loaded, InGame, Left }`; `pub struct PeerId(pub u16)`.
//! - `pub struct SessionConfig { load_timeout_ms: u64 (120_000), intro_ms: u64 (0 = no intro), countdown_ticks: u32
//!   (180) }` with `pub` fields and `Default`.
//! - `pub struct Session` with:
//!   - `new(config)`, starting in `Lobby` with no peers; `phase()`, `peer(id) -> Option<PeerState>`, `peers()` iterator in
//!     id order;
//!   - `join(id)`: a new peer enters `Lobby` if the phase is `Lobby`, otherwise `InGame` only while `Running` (join in
//!     progress) and is refused (`SessionError::Closed`) in the other phases; a known id is `SessionError::Duplicate`;
//!   - `leave(id)`: the peer becomes `Left` and is removed after the event is queued;
//!   - `start_loading(now_ms)`: `Lobby` → `Loading`; every peer → `Loading`;
//!   - `host_loaded(now_ms)`: `Loading` → `WaitLoads`; the load deadline is `now_ms + load_timeout_ms`;
//!   - `peer_loaded(id)`: the peer → `Loaded`;
//!   - `update(&mut self, now_ms: u64, tick: u32)`: in `WaitLoads`, when every peer is `Loaded` (or at the deadline, after
//!     dropping the others with `SessionEvent::PeerDropped(id)`) → `Intro`, or straight to `Countdown` when `intro_ms` is
//!     0; `Intro` ends after `intro_ms` → `Countdown { start_tick: tick + countdown_ticks }`; `Countdown` → `Running` when
//!     `tick >= start_tick` (all `Loaded` peers → `InGame`);
//!   - `finish()`: `Running` → `Results`; `back_to_lobby()`: `Results` → `Lobby`, peers → `Lobby`;
//!   - every phase change queues `SessionEvent::Phase(phase)`; `poll_event(&mut self) -> Option<SessionEvent>`;
//!   - a call that does not fit the current phase is `Err(SessionError::WrongPhase { phase })` and changes nothing.
//! - Tests: the full path with 3 peers; a peer that never loads is dropped at exactly the deadline; join refused while
//!   loading; join in progress; every wrong call in every phase leaves the state unchanged.
