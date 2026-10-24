//! The admission of one `Auth` plus `Join` ([`nfsmw_net::join::JoinGate`]): a seat in the lobby, a resumed slot, or
//! a `JoinRejected` and a close.

use blackbox_net_core::endpoint::{PeerLimits, reason};
use blackbox_net_core::session::PeerId;
use nfsmw_net::drop::{RESUME_TOKEN_BYTES, on_resume};
use nfsmw_net::input::InputWindow;
use nfsmw_net::join::{Admission, JoinRequest, PasswordProof, ServerView};
use nfsmw_net::lobby::NewMember;
use nfsmw_net::message::{
    CAP_SPECTATE_ONLY, Join, JoinAccepted, JoinRejected, Message, Role, ServerCaps, SessionPhase,
};
use nfsmw_net::roles::Rank;
use nfsmw_net::spectate::{RateClass, snapshot_hz};
use nfsmw_net::version::{Admitted, ClientOffer, PROTO_MAJOR, ProtoVersion, Refusal, RejectReason};

use super::Server;
use super::peer::Stage;
use crate::catalog::CatalogRef;
use crate::control::WorldControl;

/// `JoinAccepted.entity_id` of a player without a car yet (in the lobby, or a spectator).
pub const NO_ENTITY: u16 = u16::MAX;
/// The inbound budget of a seated player: twice the 4 KB/s client target of features.md section 4, and room for an
/// input, a ping and a few reliable messages per tick.
const SEATED_LIMITS: PeerLimits = PeerLimits { bytes_per_s: 8 * 1024, datagrams_per_tick: 6 };
/// How long a refused or kicked connection stays open so its last message gets out.
pub(super) const CLOSE_DELAY_MS: u64 = 250;

impl<W: WorldControl> Server<W> {
    /// `Join` from an authenticated connection.
    pub(super) fn on_join(&mut self, peer: PeerId, join: Join, password: PasswordProof) {
        let now = self.now_ms;
        let Some(state) = self.peers.get(&peer) else {
            return;
        };
        let Some(auth) = state.auth.clone() else {
            return;
        };
        let player_id = state.key.0;
        let offer = ClientOffer {
            proto: ProtoVersion { major: PROTO_MAJOR, minor: auth.proto_minor },
            sim_build: auth.sim_build,
            data_id: auth.data_id,
            mods: &join.mod_manifest,
        };
        let request = JoinRequest {
            source: state.source,
            player_id,
            offer,
            caps: auth.caps,
            password,
            resume: auth.resume.as_deref(),
            loadout: &join.loadout,
        };
        let view = ServerView {
            build: &self.setup.build,
            policy: self.setup.policy,
            settings: self.lobby.settings(),
            phase: self.lobby.wire_phase(),
            racers: self.lobby.racers(),
            spectators: self.lobby.spectators(),
            shutting_down: self.shutting_down,
        };
        let catalog = CatalogRef(self.setup.catalog.as_ref());
        let admission = self.gate.admit(&request, &view, self.lobby.bans(), &mut self.book, &catalog, now);
        match admission {
            Err(refusal) => self.reject(peer, refusal),
            Ok(Admission::Seat { placement, admitted }) => {
                let member = NewMember {
                    player_id,
                    name: join.display_name,
                    rank: Rank::Player,
                    placement,
                    spectate_only: auth.caps & CAP_SPECTATE_ONLY != 0,
                    loadout: join.loadout,
                };
                match self.lobby.add(member, now) {
                    Ok((slot, actions)) => {
                        self.seat(peer, slot, admitted);
                        self.apply_actions(actions);
                        if self.lobby.wire_phase() != SessionPhase::Lobby {
                            self.start_bulk(slot);
                        }
                    }
                    Err(err) => {
                        log::debug!("connection {}: lobby refused the seat: {err:?}", peer.0);
                        self.reject(peer, Refusal { reason: RejectReason::ServerFull, detail: 0 });
                    }
                }
            }
            Ok(Admission::Resume { resumed, admitted }) => {
                if let Some(old) = self.slots.get(&resumed.slot).copied() {
                    self.close(old, reason::NORMAL, 0);
                }
                self.seat(peer, resumed.slot, admitted);
                self.send_to_slot(resumed.slot, Message::LobbyState(self.lobby.state()));
                let has_car = self.frames.car_of(resumed.slot).is_some();
                self.apply_drop_actions(on_resume(resumed.slot, has_car));
                log::info!("slot {} resumed by connection {}", resumed.slot, peer.0);
            }
        }
    }

    /// Answers `JoinRejected` and closes the connection shortly after.
    fn reject(&mut self, peer: PeerId, refusal: Refusal) {
        self.stats.rejected += 1;
        log::info!("connection {} refused: {:?} ({})", peer.0, refusal.reason, refusal.detail);
        let build = &self.setup.build;
        let rejected = JoinRejected {
            reason: refusal.reason,
            detail: refusal.detail,
            server_proto: build.proto,
            server_data_id: build.data_id,
        };
        self.send_to_peer(peer, Message::JoinRejected(rejected));
        self.close(peer, reason::NORMAL, CLOSE_DELAY_MS);
    }

    /// Seats connection `peer` in `slot` and answers `JoinAccepted` with a fresh resume token.
    fn seat(&mut self, peer: PeerId, slot: u8, admitted: Admitted) {
        let now = self.now_ms;
        let tick = self.world.tick();
        let mut fresh = [0u8; RESUME_TOKEN_BYTES];
        if getrandom::fill(&mut fresh).is_err() {
            log::error!("no randomness for a resume token; slot {slot} cannot resume");
        }
        let Some(state) = self.peers.get_mut(&peer) else {
            return;
        };
        state.stage = Stage::Joined(slot);
        state.window = InputWindow::new(tick);
        state.resync();
        let player_id = state.key.0;
        self.slots.insert(slot, peer);
        self.presence.heard(slot, now);
        self.endpoint.set_budget(peer, SEATED_LIMITS);
        self.stats.joins += 1;
        let role = self.lobby.member(slot).map_or(Role::Spectator, |member| member.role);
        let resume_token = self.book.issue(slot, player_id, fresh);
        let accepted = JoinAccepted {
            player_id,
            slot,
            entity_id: self.frames.car_of(slot).unwrap_or(NO_ENTITY),
            session_seed: self.setup.session_seed,
            tick,
            snapshot_hz: snapshot_hz(RateClass::of(role), self.config.snapshot_hz),
            settings: self.lobby.settings().clone(),
            world_half_extent: self.setup.schema.half_extent(),
            resume_token,
            server_caps: ServerCaps { flags: 0, schema_id: self.setup.schema.schema_id() },
            proto_minor: admitted.minor,
        };
        self.send_to_peer(peer, Message::JoinAccepted(accepted));
        log::info!("connection {} seated in slot {slot} as {role:?}", peer.0);
    }
}
