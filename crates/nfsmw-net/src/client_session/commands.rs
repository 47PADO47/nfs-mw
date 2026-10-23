//! What the game asks of the session: the owner's input each tick, and the lobby and session commands.

use std::ops::RangeInclusive;

use blackbox_vehicle::InputState;
use nfsmw_sim::Tick;

use super::ClientLink;
use super::session::ClientSession;
use super::status::EndReason;
use crate::input::{FRAME_INPUT, MAX_INPUT_PAYLOAD};
use crate::loadout::Loadout;
use crate::message::{
    AdminCommand, ChatChannel, ChatSend, ChatText, HostCommand, LimitError, LoadComplete, QuickMessage, ResetRequest,
    ResultRequest, Role, SelectLoadout, SetReady, SetRole, SettingsPatch, StreamingStall, UpdateSettings, Vote,
    VoteKind,
};
use crate::predict::PredictedCar;
use crate::spectate::Camera;

impl<L: ClientLink, C: PredictedCar + Clone> ClientSession<L, C> {
    /// The ticks the owner's car should be stepped for now, oldest first: up to `floor(S_est + lead)`. `None` before
    /// the first own block, while reconnecting, or for a spectator.
    pub fn due_ticks(&mut self) -> Option<RangeInclusive<Tick>> {
        if !self.status.is_joined() || self.is_spectator() {
            return None;
        }
        let server_tick = self.clock.server_tick()?;
        self.own.due(server_tick)
    }

    /// Steps the owner's car for `tick` with the player's `input` (quantised first, held neutral before the race
    /// start) and records it for sending. Only the next due tick is taken (`false` otherwise). The INPUT frame goes
    /// out with the last due tick, or at the next [`ClientSession::update`].
    pub fn submit_input(&mut self, tick: Tick, input: &InputState) -> bool {
        if !self.status.is_joined() || self.is_spectator() {
            return false;
        }
        if !self.own.step(tick, input) {
            return false;
        }
        let last = self.clock.server_tick().and_then(|server_tick| self.own.target_tick(server_tick));
        if last.is_none_or(|last| tick >= last) {
            self.flush_input();
        }
        true
    }

    /// Steps every due tick with `input(tick)` and sends the frame: the whole per-frame input path in one call.
    pub fn drive(&mut self, mut input: impl FnMut(Tick) -> InputState) {
        let Some(ticks) = self.due_ticks() else {
            return;
        };
        for tick in ticks {
            if !self.submit_input(tick, &input(tick)) {
                break;
            }
        }
        self.flush_input();
    }

    /// Sends the INPUT frame of the inputs recorded since the last one (the newest unacknowledged ticks, up to 8).
    pub(super) fn flush_input(&mut self) {
        let Some(frame) = self.own.take_frame() else {
            return;
        };
        let mut buf = [0u8; MAX_INPUT_PAYLOAD];
        let Ok(len) = frame.encode(&mut buf) else {
            return;
        };
        self.counters.inputs_sent += 1;
        self.send_frame(FRAME_INPUT, buf[..len].to_vec());
    }

    /// Ready or not in the lobby.
    pub fn set_ready(&mut self, ready: bool) -> bool {
        self.send(SetReady { ready }.into())
    }

    /// Asks for another car; also the car of a later reconnect's `Join`.
    pub fn choose_loadout(&mut self, loadout: Loadout) -> bool {
        self.config.loadout = loadout.clone();
        self.send(SelectLoadout { loadout }.into())
    }

    /// Asks for a role (racer, cop or spectator); the server checks it against the settings.
    pub fn set_role(&mut self, role: Role) -> bool {
        self.send(SetRole { role }.into())
    }

    /// Says `text` to everybody. [`LimitError`] for empty, over-long or forbidden text.
    pub fn say(&mut self, text: &str) -> Result<bool, LimitError> {
        let text = ChatText::new(text)?;
        Ok(self.send(ChatSend { channel: ChatChannel::All, to_slot: None, text }.into()))
    }

    /// Says `text` to the player in `slot` only.
    pub fn whisper(&mut self, slot: u8, text: &str) -> Result<bool, LimitError> {
        let text = ChatText::new(text)?;
        Ok(self.send(ChatSend { channel: ChatChannel::Whisper, to_slot: Some(slot), text }.into()))
    }

    /// Sends preset line `id` (MP-44).
    pub fn quick_message(&mut self, id: u16) -> bool {
        self.send(QuickMessage { id }.into())
    }

    /// Votes for (or starts a vote on) `option` of `kind`.
    pub fn vote(&mut self, kind: VoteKind, option: u32) -> bool {
        self.send(Vote { kind, option }.into())
    }

    /// Asks the server to reset the car; the answer is a `Respawn` event and a `reset` own block.
    pub fn request_reset(&mut self) -> bool {
        let tick = self.clock.server_tick().map_or(0, |tick| tick.max(0.0) as Tick);
        self.send(ResetRequest { tick }.into())
    }

    /// The world for `LoadWorld` is loaded (and its collision streamed when `streaming_ready`).
    pub fn load_complete(&mut self, streaming_ready: bool) -> bool {
        self.send(LoadComplete { streaming_ready }.into())
    }

    /// The client's streaming fell behind (`true`) or caught up.
    pub fn streaming_stall(&mut self, stalled: bool) -> bool {
        self.send(StreamingStall { stalled }.into())
    }

    /// Host or admin: change settings.
    pub fn update_settings(&mut self, patch: SettingsPatch) -> bool {
        self.send(UpdateSettings { patch }.into())
    }

    /// Host or admin: kick, ban, mute, restart or change mode.
    pub fn host_command(&mut self, command: AdminCommand) -> bool {
        self.send(HostCommand { command }.into())
    }

    /// Asks for the last result record again (after a reconnect).
    pub fn request_results(&mut self) -> bool {
        self.send(ResultRequest {}.into())
    }

    /// Leaves the session: the server frees the slot at once.
    pub fn leave(&mut self) {
        self.end(EndReason::Left);
    }

    /// Chooses the spectator camera. Following a slot that is not a racer is refused.
    pub fn follow(&mut self, camera: Camera) -> bool {
        let racers = self.lobby.racers();
        self.camera.set(camera, &racers)
    }

    /// The spectator camera follows the next racer, wrapping.
    pub fn follow_next(&mut self) {
        let racers = self.lobby.racers();
        self.camera.next(&racers);
    }

    /// The spectator camera follows the previous racer, wrapping.
    pub fn follow_previous(&mut self) {
        let racers = self.lobby.racers();
        self.camera.previous(&racers);
    }
}
