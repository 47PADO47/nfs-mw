//! Join-in-progress policy (MP-23) and the server's admission pipeline for one `Auth` plus `Join`.
//!
//! **Placement** ([`placement`]). In the lobby every joiner is a racer. Once a session has started, the setting
//! `join_in_progress` decides: `Closed` refuses with `InProgress`; `FreeRoam` seats a racer straight into a running
//! free roam; otherwise (a race, or a race loading, or its results) the joiner becomes a spectator who waits for the next
//! race and is promoted when the session returns to the lobby. A spectate-only client is always a spectator.
//!
//! **Admission** ([`JoinGate::admit`]), in this order, each step with the `JoinRejected` reason of [`crate::version`]:
//!
//! 1. joins per source address, 5 per 10 s (`RateLimited`; a storm past the limit should get no answer at all);
//! 2. the server shutting down (`ServerShuttingDown`);
//! 3. [`check_join`]: the protocol versions (`ProtocolMismatch` naming the side that must update), the game data
//!    (`DataMismatch`), the sim build when the settings require it (`SimBuildMismatch`), the mods (`ModMismatch`);
//! 4. the session's bans (`Banned`);
//! 5. the password, when the room has one: the caller reports the PAKE outcome; a missing or wrong password counts a
//!    guess and, like a guess limit, answers with the generic `ServerFull` (security.md 4.6: nothing is revealed);
//! 6. a resume token: a valid one returns the held slot ([`Admission::Resume`]) past every later check; an unknown or
//!    expired one makes this a normal join (MP-51);
//! 7. the policy above (`InProgress`);
//! 8. the limits: racers against `max_players`, spectators against `max_spectators`, counted apart (`ServerFull`);
//! 9. the loadout of a racer, or of a spectator waiting to race, against the catalogue and the session's car
//!    restriction (`LoadoutRejected` with the loadout reason as the detail).

mod guard;

#[cfg(test)]
mod tests;

pub use guard::{
    GUESS_LIMIT, GUESS_WINDOW_MS, JoinGuard, LOCK_MS, MAX_GUESS_KEYS, MAX_LOCK_MS, MAX_SOURCES, PasswordGuard,
};

use crate::drop::{ResumeBook, Resumed};
use crate::loadout::{Catalog, Loadout, validate};
use crate::message::{CAP_SPECTATE_ONLY, JoinInProgress, Mode, SessionPhase, Settings};
use crate::roles::Bans;
use crate::spectate::{SpectateReason, may_spectate};
use crate::version::{Admitted, BuildInfo, ClientOffer, Refusal, RejectReason, ServerPolicy, check_join};

/// Where a joiner is seated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Placement {
    Racer,
    Spectator(SpectateReason),
}

/// The seat for a joiner of a session in `phase`. `spectate_only` is the client's `CAP_SPECTATE_ONLY`.
pub fn placement(settings: &Settings, phase: SessionPhase, spectate_only: bool) -> Result<Placement, RejectReason> {
    if spectate_only {
        return Ok(Placement::Spectator(SpectateReason::Chosen));
    }
    if phase == SessionPhase::Lobby {
        return Ok(Placement::Racer);
    }
    let free_roam_running = phase == SessionPhase::Running && settings.mode == Mode::FreeRoam;
    match settings.join_in_progress {
        JoinInProgress::Closed => Err(RejectReason::InProgress),
        JoinInProgress::FreeRoam if free_roam_running => Ok(Placement::Racer),
        JoinInProgress::FreeRoam | JoinInProgress::BetweenRaces => {
            Ok(Placement::Spectator(SpectateReason::WaitingForRace))
        }
    }
}

/// What the room password exchange (PAKE, run by the transport) concluded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PasswordProof {
    /// The client did not run it.
    Missing,
    Passed,
    Failed,
}

/// One joiner, from its `Auth` and `Join`.
#[derive(Debug, Clone, Copy)]
pub struct JoinRequest<'a> {
    /// The caller's key for the source address (a hash of the IP and port, for instance).
    pub source: u64,
    /// The hash of the identity key the handshake authenticated.
    pub player_id: [u8; 16],
    pub offer: ClientOffer<'a>,
    /// `Auth.caps`.
    pub caps: u8,
    pub password: PasswordProof,
    /// `Auth.resume`.
    pub resume: Option<&'a [u8]>,
    pub loadout: &'a Loadout,
}

/// What the server is, for the admission.
#[derive(Debug, Clone, Copy)]
pub struct ServerView<'a> {
    pub build: &'a BuildInfo,
    /// The version policy; the settings' `sim_build_match` turns its sim-build check on as well.
    pub policy: ServerPolicy,
    pub settings: &'a Settings,
    pub phase: SessionPhase,
    /// Racers and cops in the session, held slots included.
    pub racers: usize,
    pub spectators: usize,
    pub shutting_down: bool,
}

/// An admitted joiner.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Admission {
    /// A new member: add it to the lobby with this placement.
    Seat { placement: Placement, admitted: Admitted },
    /// The holder of a held slot came back: restore it ([`crate::drop::on_resume`]) and issue a new token.
    Resume { resumed: Resumed, admitted: Admitted },
}

/// The admission state kept across joins: the storm limit and the password guard.
#[derive(Debug, Clone, Default)]
pub struct JoinGate {
    joins: JoinGuard,
    passwords: PasswordGuard,
}

impl JoinGate {
    pub fn new() -> Self {
        Self::default()
    }

    /// The decision for `request` at `now_ms`, in the order of the module docs. A resume uses up its token.
    pub fn admit(
        &mut self,
        request: &JoinRequest<'_>,
        view: &ServerView<'_>,
        bans: &Bans,
        book: &mut ResumeBook,
        catalog: &impl Catalog,
        now_ms: u64,
    ) -> Result<Admission, Refusal> {
        if !self.joins.allow(request.source, now_ms) {
            return Err(refuse(RejectReason::RateLimited, 0));
        }
        if view.shutting_down {
            return Err(refuse(RejectReason::ServerShuttingDown, 0));
        }
        let settings = view.settings;
        let mut policy = view.policy;
        policy.require_sim_build |= settings.sim_build_match;
        let admitted = check_join(view.build, &policy, &settings.required_mods, &request.offer)?;
        if bans.is_banned(request.player_id, now_ms) {
            return Err(refuse(RejectReason::Banned, 0));
        }
        if settings.password_required {
            self.check_password(request, now_ms)?;
        }
        if let Some(token) = request.resume
            && let Ok(resumed) = book.resume(token, request.player_id, now_ms)
        {
            return Ok(Admission::Resume { resumed, admitted });
        }
        let spectate_only = request.caps & CAP_SPECTATE_ONLY != 0;
        let placement = placement(settings, view.phase, spectate_only).map_err(|reason| refuse(reason, 0))?;
        check_limits(placement, view)?;
        if placement != Placement::Spectator(SpectateReason::Chosen)
            && let Err(reject) = validate(request.loadout, catalog, &settings.allowed_cars)
        {
            return Err(refuse(RejectReason::LoadoutRejected, reject.detail()));
        }
        Ok(Admission::Seat { placement, admitted })
    }

    fn check_password(&mut self, request: &JoinRequest<'_>, now_ms: u64) -> Result<(), Refusal> {
        if self.passwords.locked(request.player_id, request.source, now_ms) {
            return Err(generic());
        }
        if request.password != PasswordProof::Passed {
            self.passwords.failed(request.player_id, request.source, now_ms);
            return Err(generic());
        }
        self.passwords.passed(request.player_id);
        Ok(())
    }
}

fn check_limits(placement: Placement, view: &ServerView<'_>) -> Result<(), Refusal> {
    let Placement::Spectator(reason) = placement else {
        if view.racers >= usize::from(view.settings.max_players) {
            return Err(refuse(RejectReason::ServerFull, 0));
        }
        return Ok(());
    };
    may_spectate(view.settings, view.spectators, reason).map_err(|reason| refuse(reason, 0))
}

/// The answer for a failed password or a guess limit: the same as a full room.
fn generic() -> Refusal {
    refuse(RejectReason::ServerFull, 0)
}

fn refuse(reason: RejectReason, detail: u16) -> Refusal {
    Refusal { reason, detail }
}
