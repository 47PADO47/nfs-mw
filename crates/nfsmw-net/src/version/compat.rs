//! The compatibility decision (MP-60 to MP-64, protocol.md sections 8.1 and 8.4).
//!
//! The server checks, in order: the protocol major (a mismatch is a hard reject naming the side that must update), the
//! minor against the oldest it still serves, the data id, the sim build when the server requires it, then the mods.
//! A newer client is never refused for its minor: it runs at the server's minor and turns off what that minor lacks.

use super::RejectReason;
use super::ids::{DataId, ProtoVersion, SimBuild};
use super::mods::{ModEntry, ModPolicy, compare_mods};

/// `detail` of a `ProtocolMismatch`: the client is older than the server serves and must update.
pub const DETAIL_CLIENT_MUST_UPDATE: u16 = 1;
/// `detail` of a `ProtocolMismatch`: the server is older than the client's major and must update.
pub const DETAIL_SERVER_MUST_UPDATE: u16 = 2;

/// What one build is: its protocol, its sim build and the game data it runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BuildInfo {
    pub proto: ProtoVersion,
    pub sim_build: SimBuild,
    pub data_id: DataId,
}

/// The server's compatibility settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct ServerPolicy {
    /// The oldest minor the server still serves; clamped to the server's own minor.
    pub min_minor: u16,
    /// Refuse a client of another sim build (competitive sessions, MP-62).
    pub require_sim_build: bool,
    /// How the mod lists are compared.
    pub mods: ModPolicy,
}

/// What a joining client presents: its `Auth` fields and the manifest of its `Join`.
#[derive(Debug, Clone, Copy)]
pub struct ClientOffer<'a> {
    pub proto: ProtoVersion,
    pub sim_build: SimBuild,
    pub data_id: DataId,
    pub mods: &'a [ModEntry],
}

/// A client the server lets in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Admitted {
    /// The minor both sides use: the lower of the two.
    pub minor: u16,
    /// Whether the sim builds are equal, so prediction should be exact.
    pub sim_build_matches: bool,
}

/// Why a client is refused, as `JoinRejected` carries it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Refusal {
    pub reason: RejectReason,
    /// A reason-specific code, never free text: [`DETAIL_CLIENT_MUST_UPDATE`] or [`DETAIL_SERVER_MUST_UPDATE`] for a
    /// protocol mismatch, the number of differences for a mod mismatch, 0 otherwise.
    pub detail: u16,
}

impl Refusal {
    fn new(reason: RejectReason, detail: u16) -> Self {
        Self { reason, detail }
    }
}

/// The server's decision for a joining client.
pub fn check_join(
    server: &BuildInfo,
    policy: &ServerPolicy,
    required_mods: &[ModEntry],
    client: &ClientOffer<'_>,
) -> Result<Admitted, Refusal> {
    if client.proto.major < server.proto.major {
        return Err(Refusal::new(RejectReason::ProtocolMismatch, DETAIL_CLIENT_MUST_UPDATE));
    }
    if client.proto.major > server.proto.major {
        return Err(Refusal::new(RejectReason::ProtocolMismatch, DETAIL_SERVER_MUST_UPDATE));
    }
    let oldest = policy.min_minor.min(server.proto.minor);
    if client.proto.minor < oldest {
        return Err(Refusal::new(RejectReason::ProtocolMismatch, DETAIL_CLIENT_MUST_UPDATE));
    }
    if client.data_id != server.data_id {
        return Err(Refusal::new(RejectReason::DataMismatch, 0));
    }
    let sim_build_matches = client.sim_build == server.sim_build;
    if policy.require_sim_build && !sim_build_matches {
        return Err(Refusal::new(RejectReason::SimBuildMismatch, 0));
    }
    if let Err(diff) = compare_mods(required_mods, client.mods, policy.mods) {
        return Err(Refusal::new(RejectReason::ModMismatch, diff.detail()));
    }
    Ok(Admitted { minor: client.proto.minor.min(server.proto.minor), sim_build_matches })
}

/// The minor a client uses with a server that reported `server` in `JoinAccepted`, or `None` when the majors differ.
pub fn negotiate_minor(local: ProtoVersion, server: ProtoVersion) -> Option<u16> {
    if local.major != server.major {
        return None;
    }
    Some(local.minor.min(server.minor))
}

/// An optional protocol feature that a later minor adds. A side turns it on only when the negotiated minor has it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Feature {
    /// The `SPECTATOR_HINT` frame (protocol.md section 5.2).
    SpectatorHints,
    /// The `VOICE` frame (MP-43).
    Voice,
    /// The `TELEMETRY_DEBUG` frame (MP-85).
    TelemetryDebug,
}

impl Feature {
    /// The first minor that has the feature, or `None` while no released minor defines it.
    pub fn since_minor(self) -> Option<u16> {
        match self {
            Self::SpectatorHints | Self::Voice | Self::TelemetryDebug => None,
        }
    }

    /// Whether the feature is on at the negotiated `minor`.
    pub fn enabled_at(self, minor: u16) -> bool {
        self.since_minor().is_some_and(|since| minor >= since)
    }
}

/// How the server browser shows a discovery reply.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ServerLabel {
    /// Same major and the same data prefix: joinable as far as discovery can tell.
    Compatible,
    /// Another major: greyed out; `detail` is [`DETAIL_CLIENT_MUST_UPDATE`] or [`DETAIL_SERVER_MUST_UPDATE`].
    VersionMismatch { detail: u16 },
    /// Another game data set.
    DataMismatch,
}

/// The label of a discovery reply with protocol major `major` and data id prefix `data_short`. The reply is only a
/// hint; the join repeats every check.
pub fn classify_server(local: &BuildInfo, major: u8, data_short: [u8; 4]) -> ServerLabel {
    if major > local.proto.major {
        return ServerLabel::VersionMismatch { detail: DETAIL_CLIENT_MUST_UPDATE };
    }
    if major < local.proto.major {
        return ServerLabel::VersionMismatch { detail: DETAIL_SERVER_MUST_UPDATE };
    }
    if data_short != local.data_id.short() {
        return ServerLabel::DataMismatch;
    }
    ServerLabel::Compatible
}
