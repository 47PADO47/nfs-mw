//! A minimal test client built from the raw protocol: `nfsmw_net::message` for the reliable messages, the input
//! frame codec for `INPUT`, and the server's frame codecs for `SNAPSHOT`, `PING` and `PONG`. No session driver.

use blackbox_net_core::bits::BitReader;
use blackbox_net_core::endpoint::{ClientEndpoint, ClientEvent, DisconnectReason};
use blackbox_net_core::replicate::ClientViews;
use blackbox_net_core::secure::IdentityKey;
use blackbox_net_core::secure::password::{self, PakeConfirm, PakeRole};
use nfsmw_net::clock::{Ping, Pong};
use nfsmw_net::frames::{FRAME_PING, FRAME_PONG, FRAME_SNAPSHOT, GlobalBlock, decode_pong, encode_ping, read_snapshot};
use nfsmw_net::input::{FRAME_INPUT, InputCode, InputHistory, MAX_INPUT_PAYLOAD, expand_tick};
use nfsmw_net::loadout::Loadout;
use nfsmw_net::message::{
    Auth, Bytes, Join, JoinAccepted, JoinRejected, Kick, LoadComplete, Message, Name, PakeMsg, SetReady, decode, encode,
};
use nfsmw_net::schema::GameSchema;
use nfsmw_net::schema::OwnBlock;
use nfsmw_net::version::ModList;
use nfsmw_server::toy_build;

/// A fixed identity from one seed byte.
pub fn identity(seed: u8) -> IdentityKey {
    IdentityKey::from_secret(&[seed; 32]).expect("any 32 bytes are a key")
}

/// One decoded snapshot.
#[derive(Debug, Clone, PartialEq)]
pub struct Snap {
    pub tick: u32,
    pub ack_input: u16,
    pub own: Option<OwnBlock>,
    pub global: Option<GlobalBlock>,
    /// 0 for a full snapshot.
    pub baseline_distance: u32,
}

/// One raw client.
pub struct TestClient {
    pub endpoint: ClientEndpoint,
    pub schema: GameSchema,
    pub accepted: Option<JoinAccepted>,
    pub rejected: Option<JoinRejected>,
    pub kicked: Option<Kick>,
    pub disconnected: Option<DisconnectReason>,
    /// Every reliable message received, in order.
    pub messages: Vec<Message>,
    pub views: ClientViews,
    pub snapshots: Vec<Snap>,
    /// Bytes of every snapshot payload.
    pub snapshot_bytes: u64,
    pub snapshot_errors: u32,
    pub pongs: Vec<Pong>,
    pub history: InputHistory,
    /// The newest input tick the server acknowledged.
    pub acked: Option<u32>,
    /// Stops sending inputs and pings (a client that went silent).
    pub silent: bool,
    /// The room password to answer a `PakeStart` with; `None` ignores it.
    pub password: Option<String>,
    /// The Noise handshake hash the connection reported (zero on the in-memory link).
    pub handshake_hash: [u8; 32],
    /// This side's confirmation of the exchange, until the server's tag arrives.
    pake: Option<PakeConfirm>,
    /// Whether the server's confirmation tag matched, once it came.
    pub server_proved: Option<bool>,
}

impl TestClient {
    /// A client on `endpoint`.
    pub fn new(endpoint: ClientEndpoint) -> Self {
        Self {
            endpoint,
            schema: GameSchema::new(),
            accepted: None,
            rejected: None,
            kicked: None,
            disconnected: None,
            messages: Vec::new(),
            views: ClientViews::new(),
            snapshots: Vec::new(),
            snapshot_bytes: 0,
            snapshot_errors: 0,
            pongs: Vec::new(),
            history: InputHistory::new(false),
            acked: None,
            silent: false,
            password: None,
            handshake_hash: [0; 32],
            pake: None,
            server_proved: None,
        }
    }

    /// The seated slot.
    pub fn slot(&self) -> Option<u8> {
        self.accepted.as_ref().map(|accepted| accepted.slot)
    }

    /// Sends one reliable message on its channel.
    pub fn send(&mut self, message: Message) {
        let mut bytes = Vec::new();
        encode(&message, &mut bytes).expect("the message fits");
        self.endpoint.send_reliable(message.channel().index(), bytes);
    }

    /// `Auth` with the toy build, optionally resuming, then `Join` with a stock car of type `car`.
    pub fn join(&mut self, name: &str, car: u32, resume: Option<Vec<u8>>, caps: u8) {
        let build = toy_build();
        let resume = resume.map(|token| Bytes::new(token).expect("a token fits"));
        let auth = Auth {
            proto_minor: build.proto.minor,
            sim_build: build.sim_build,
            data_id: build.data_id,
            caps,
            connect_token: None,
            resume,
        };
        self.send(Message::Auth(auth));
        let join = Join {
            display_name: Name::new(name).expect("a valid name"),
            loadout: Loadout::stock(car),
            mod_manifest: ModList::empty(),
        };
        self.send(Message::Join(join));
    }

    /// `SetReady`.
    pub fn ready(&mut self, ready: bool) {
        self.send(Message::SetReady(SetReady { ready }));
    }

    /// Records `code` for `tick` and sends the input frame of every unacknowledged tick.
    pub fn drive(&mut self, tick: u32, code: InputCode) {
        if self.silent {
            return;
        }
        self.history.record(tick, code);
        let Some(frame) = self.history.frame(self.acked, 8) else {
            return;
        };
        let mut buf = [0u8; MAX_INPUT_PAYLOAD];
        let len = frame.encode(&mut buf).expect("the frame encodes");
        self.endpoint.send_unreliable(FRAME_INPUT, buf[..len].to_vec(), None);
    }

    /// Sends a `PING`.
    pub fn ping(&mut self, seq: u8, time_ms: u32) {
        if self.silent {
            return;
        }
        self.endpoint.send_unreliable(FRAME_PING, encode_ping(&Ping { seq, client_time_ms: time_ms }).to_vec(), None);
    }

    /// Reads every event the net thread queued.
    pub fn pump(&mut self) {
        while let Some(event) = self.endpoint.poll_event() {
            match event {
                ClientEvent::Reliable { bytes, .. } => self.on_reliable(&bytes),
                ClientEvent::Unreliable { frame_type, bytes } => self.on_unreliable(frame_type, &bytes),
                ClientEvent::Disconnected(reason) => self.disconnected = Some(reason),
                ClientEvent::Connected { handshake_hash, .. } => self.handshake_hash = handshake_hash,
                _ => {}
            }
        }
    }

    fn on_reliable(&mut self, bytes: &[u8]) {
        let message = decode(bytes).expect("the server sends valid messages");
        match &message {
            Message::JoinAccepted(accepted) => self.accepted = Some(accepted.clone()),
            Message::JoinRejected(rejected) => self.rejected = Some(rejected.clone()),
            Message::Kick(kick) => self.kicked = Some(kick.clone()),
            Message::LoadWorld(_) => self.send(Message::LoadComplete(LoadComplete { streaming_ready: true })),
            Message::PakeStart(start) => self.answer_pake(&start.message),
            Message::PakeConfirm(tag) => {
                self.server_proved = Some(self.pake.take().is_some_and(|confirm| confirm.verify(&tag.tag)));
            }
            _ => {}
        }
        self.messages.push(message);
    }

    /// The room has a password: answers with this side's message and tag, when a password was given.
    fn answer_pake(&mut self, server_message: &[u8]) {
        let Some(secret) = self.password.clone() else {
            return;
        };
        let (state, mine) =
            password::start(secret.as_bytes(), &self.handshake_hash, PakeRole::Joiner).expect("a usable password");
        let confirm = state.finish(server_message).expect("the server's message is a point");
        let message = Bytes::new(mine.to_vec()).expect("a PAKE message fits");
        self.send(Message::PakeMsg(PakeMsg { message }));
        self.send(Message::PakeConfirm(nfsmw_net::message::PakeConfirm { tag: confirm.tag() }));
        self.pake = Some(confirm);
    }

    fn on_unreliable(&mut self, frame_type: u8, bytes: &[u8]) {
        if frame_type == FRAME_PONG {
            self.pongs.push(decode_pong(bytes).expect("a valid pong"));
            return;
        }
        if frame_type != FRAME_SNAPSHOT {
            return;
        }
        self.snapshot_bytes += bytes.len() as u64;
        let Some(decoded) = self.decode(bytes) else {
            self.snapshot_errors += 1;
            return;
        };
        if decoded.own.is_some() {
            let ack = expand_tick(decoded.ack_input, decoded.tick);
            self.acked = Some(self.acked.map_or(ack, |acked| acked.max(ack)));
        }
        self.snapshots.push(decoded);
    }

    /// One single-part snapshot decoded against the client's views.
    fn decode(&mut self, bytes: &[u8]) -> Option<Snap> {
        let frame = read_snapshot(&self.schema, bytes).ok()?;
        if frame.header.parts != 1 {
            return None;
        }
        let mut peek = BitReader::new(frame.body);
        peek.read_bits(32).ok()?;
        let baseline_distance = peek.read_varint().ok()? as u32;
        let mut r = BitReader::new(frame.body);
        let tick = self.views.read_snapshot(self.schema.schema(), &mut r).ok()?;
        let padding = r.remaining_bits();
        if padding >= 8 || r.read_bits(padding as u32).ok()? != 0 {
            return None;
        }
        Some(Snap { tick, ack_input: frame.header.ack_input, own: frame.own, global: frame.global, baseline_distance })
    }

    /// Whether a message matching `pick` was received.
    pub fn got(&self, pick: impl Fn(&Message) -> bool) -> bool {
        self.messages.iter().any(pick)
    }
}
