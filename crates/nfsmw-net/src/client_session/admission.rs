//! Admission (protocol.md section 4, after the keys): `Auth` and `Join` on connect, the room password exchange, and the
//! seat `JoinAccepted` gives, including a resumed one.

use blackbox_net_core::secure::password::{self, PakeRole};

use super::ClientLink;
use super::events::SessionEvent;
use super::remote::Remotes;
use super::session::ClientSession;
use super::status::{EndReason, Status};
use crate::interp::InterpConfig;
use crate::message::{
    Auth, Bytes, Join, JoinAccepted, MAX_RESUME_TOKEN, Message, PakeConfirm, PakeMsg, PakeStart, encode,
};
use crate::netstats::NetStats;
use crate::predict::PredictedCar;
use crate::schema::GameSchema;

impl<L: ClientLink, C: PredictedCar + Clone> ClientSession<L, C> {
    /// The connection is open: present the build, the resume token when reconnecting, and the seat request. `Join`
    /// follows `Auth` at once; a room with a password answers `Auth` with `PakeStart` and holds `Join` until the
    /// exchange is confirmed.
    pub(super) fn on_connected(&mut self) {
        match self.status {
            Status::Connecting => self.status = Status::Admitting,
            Status::Reconnecting { .. } => self.seat.resuming = true,
            _ => return,
        }
        let resume = match self.seat.resuming {
            true => self.seat.resume_token.clone().and_then(|token| Bytes::<MAX_RESUME_TOKEN>::new(token).ok()),
            false => None,
        };
        let auth = Auth {
            proto_minor: self.config.proto_minor,
            sim_build: self.config.sim_build,
            data_id: self.config.data_id,
            caps: self.config.caps,
            connect_token: None,
            resume,
        };
        let join = Join {
            display_name: self.config.name.clone(),
            loadout: self.config.loadout.clone(),
            mod_manifest: self.config.mods.clone(),
        };
        self.send(auth.into());
        self.send(join.into());
    }

    /// The room has a password: answer with this side's SPAKE2 message and confirmation tag.
    pub(super) fn on_pake_start(&mut self, start: &PakeStart) {
        if !self.is_admitting() || self.pake.confirm.is_some() {
            self.bad_message();
            return;
        }
        let Some(secret) = self.config.password.clone() else {
            self.end(EndReason::PasswordRequired);
            return;
        };
        let hash = self.link.as_ref().and_then(ClientLink::handshake_hash).unwrap_or([0; 32]);
        let Ok((state, mine)) = password::start(secret.as_bytes(), &hash, PakeRole::Joiner) else {
            self.end(EndReason::PasswordRejected);
            return;
        };
        let Ok(confirm) = state.finish(&start.message) else {
            self.end(EndReason::PasswordRejected);
            return;
        };
        let Ok(message) = Bytes::new(mine.to_vec()) else {
            self.end(EndReason::PasswordRejected);
            return;
        };
        self.send(PakeMsg { message }.into());
        self.send(PakeConfirm { tag: confirm.tag() }.into());
        self.pake.confirm = Some(confirm);
    }

    /// The server's confirmation tag: it proves the server knows the password.
    pub(super) fn on_pake_confirm(&mut self, tag: &[u8; 32]) {
        let Some(confirm) = self.pake.confirm.take() else {
            self.bad_message();
            return;
        };
        if !confirm.verify(tag) {
            self.end(EndReason::PasswordRejected);
        }
    }

    /// The seat. Checks the server's schema against this build's, then starts the session (or resumes it: the slot
    /// is the same and the next snapshot is full).
    pub(super) fn on_accepted(&mut self, accepted: &JoinAccepted) {
        if !self.is_admitting() {
            self.bad_message();
            return;
        }
        if self.pake.confirm.is_some() {
            self.end(EndReason::PasswordRejected);
            return;
        }
        let Ok(schema) = GameSchema::with_half_extent(accepted.world_half_extent) else {
            self.end(EndReason::SchemaMismatch);
            return;
        };
        if schema.schema_id() != accepted.server_caps.schema_id {
            self.end(EndReason::SchemaMismatch);
            return;
        }
        let resumed = self.seat.resuming && self.seat.slot == Some(accepted.slot);
        if !resumed {
            let mut tuning = self.config.tuning;
            tuning.snapshot_hz = u32::from(accepted.snapshot_hz);
            self.remotes = Remotes::new(InterpConfig::from_tuning(&tuning));
            self.stats = NetStats::new(tuning.snapshot_interval_ticks());
        }
        self.schema = schema;
        self.seat.slot = Some(accepted.slot);
        self.seat.entity = Some(accepted.entity_id);
        self.seat.resume_token = Some(accepted.resume_token.to_vec());
        self.seat.snapshot_hz = accepted.snapshot_hz;
        self.seat.proto_minor = accepted.proto_minor.min(self.config.proto_minor);
        self.seat.resuming = false;
        self.views.reset();
        self.parts.clear();
        self.own.forget();
        self.remotes.new_connection();
        self.gate.new_connection();
        self.bulk = crate::drop::BulkReceiver::new(crate::drop::SESSION_DATA_LIMIT);
        self.lobby.my_slot = Some(accepted.slot);
        self.lobby.settings = Some(accepted.settings.clone());
        self.status = Status::Joined;
        self.events.push(SessionEvent::Admitted { slot: accepted.slot, entity: accepted.entity_id, resumed });
    }

    fn is_admitting(&self) -> bool {
        matches!(self.status, Status::Admitting | Status::Reconnecting { .. })
    }

    /// Sends a reliable message on its channel. `false` when there is no link or it does not encode.
    pub(super) fn send(&mut self, message: Message) -> bool {
        let Some(link) = self.link.as_mut() else {
            return false;
        };
        if encode(&message, &mut self.out).is_err() {
            return false;
        }
        self.stats.on_bytes_out(self.out.len(), self.now_ms);
        link.send_reliable(message.channel().index(), self.out.clone());
        true
    }
}
