//! One filled-in sample of every message, for the round-trip and garbage tests.

use crate::loadout::{CarRestriction, Loadout, Paint, PartChoice};
use crate::message::*;
use crate::version::{DataId, ModEntry, ModKind, ModList, ProtoVersion, RejectReason, SimBuild};

pub fn head(seq: u32) -> EventHead {
    EventHead { seq, tick: 1000 + seq, target_tick: 0 }
}

pub fn loadout() -> Loadout {
    Loadout {
        car_type: 0xDEAD_BEEF,
        parts: List::new(vec![PartChoice { slot: 3, part: 77 }, PartChoice { slot: 40, part: 1_000_000 }]).unwrap(),
        paint: Paint { id: 12, colours: List::new(vec![1, 2, 3]).unwrap() },
        wheel_set: 5,
        plate: Plate::new("RAZOR").unwrap(),
    }
}

pub fn mods() -> ModList {
    List::new(vec![ModEntry::new("speed.tweaks", "1.2.3", [9; 32], ModKind::Content, false).unwrap()]).unwrap()
}

pub fn settings() -> Settings {
    let mut settings = Settings::lan(ServerName::new("Rockport LAN").unwrap());
    settings.mode = Mode::Circuit;
    settings.laps = 3;
    settings.allowed_cars = CarRestriction::Cars(List::new(vec![1, 2, 3]).unwrap());
    settings.required_mods = mods();
    settings
}

fn row(slot: u8) -> LobbyPlayer {
    LobbyPlayer {
        slot,
        player_id: [slot; 16],
        name: Name::new("Razor").unwrap(),
        car_type: 42,
        ready: true,
        role: Role::Racer,
        team: 1,
        ping_ms: 35,
        flags: 2,
    }
}

/// Every message type once, admission first.
pub fn all() -> Vec<Message> {
    let mut out: Vec<Message> = vec![
        Auth {
            proto_minor: 3,
            sim_build: SimBuild::from_parts("abc", "linux"),
            data_id: DataId::compute(&[1; 32], &[("A", 1)]),
            caps: CAP_CLUTCH | CAP_WHEEL,
            connect_token: Some(Bytes::new(vec![1, 2, 3]).unwrap()),
            resume: None,
        }
        .into(),
        PakeStart { scheme: PakeScheme::Spake2, message: Bytes::new(vec![7; 33]).unwrap() }.into(),
        PakeMsg { message: Bytes::new(vec![8; 33]).unwrap() }.into(),
        PakeConfirm { tag: [5; 32] }.into(),
        Join { display_name: Name::new("Mia").unwrap(), loadout: loadout(), mod_manifest: mods() }.into(),
        JoinAccepted {
            player_id: [3; 16],
            slot: 31,
            entity_id: 4000,
            session_seed: u64::MAX,
            tick: 123_456,
            snapshot_hz: 30,
            settings: settings(),
            world_half_extent: 8192.0,
            resume_token: Bytes::new(vec![4; 32]).unwrap(),
            server_caps: ServerCaps { flags: 0, schema_id: 0x0123_4567_89AB_CDEF },
            proto_minor: 0,
        }
        .into(),
        JoinRejected {
            reason: RejectReason::LoadoutRejected,
            detail: 6,
            server_proto: ProtoVersion { major: 1, minor: 2 },
            server_data_id: DataId { bytes: [2; 16] },
        }
        .into(),
        Kick { reason: KickReason::Banned, ban_minutes: 60 }.into(),
        LobbyState {
            players: List::new(vec![row(0), row(5)]).unwrap(),
            settings: settings(),
            phase: SessionPhase::Lobby,
            host_slot: 0,
        }
        .into(),
        LobbyDelta {
            changes: List::new(vec![
                LobbyChange::Add(row(7)),
                LobbyChange::Update(row(0)),
                LobbyChange::Remove { slot: 5 },
            ])
            .unwrap(),
        }
        .into(),
        SetReady { ready: true }.into(),
        SelectLoadout { loadout: loadout() }.into(),
        SetRole { role: Role::Spectator }.into(),
        UpdateSettings {
            patch: SettingsPatch { max_players: Some(12), mode: Some(Mode::Custom(200)), ..Default::default() },
        }
        .into(),
        StartCountdown { start_tick: 9000, seconds: 3 }.into(),
        CancelCountdown { reason: CancelReason::PlayerLeft }.into(),
        LoadWorld {
            track: 17,
            mode: Mode::Sprint,
            laps: 0,
            time_of_day: 18 * 60,
            weather: 1,
            traffic_density: 40,
            cop_level_cap: 3,
            pursuits: true,
            spawn_slot: 2,
        }
        .into(),
        LoadComplete { streaming_ready: true }.into(),
        StartAt { start_tick: 9180 }.into(),
        Vote { kind: VoteKind::Kick, option: 4 }.into(),
        VoteState {
            kind: VoteKind::ChangeTrack,
            options: List::new(vec![VoteOption { id: 1, count: 2 }, VoteOption { id: 9, count: 0 }]).unwrap(),
            ends_tick: 10_000,
        }
        .into(),
        HostCommand { command: AdminCommand::Ban { slot: 3, minutes: 30 } }.into(),
        StreamingStall { stalled: false }.into(),
        ResultRequest {}.into(),
        ResetRequest { tick: 5000 }.into(),
        ChatSend { channel: ChatChannel::Whisper, to_slot: Some(4), text: ChatText::new("nice drift").unwrap() }.into(),
        ChatMessage { from_slot: 2, channel: ChatChannel::All, text: ChatText::new("gg ✓").unwrap(), tick: 77 }.into(),
        SystemNotice { code: NoticeCode::Vote, args: List::new(vec![1, 2]).unwrap() }.into(),
        QuickMessage { id: 12 }.into(),
        BulkStart { kind: BulkKind::SessionData, id: 1, length: 4096, hash: [6; 32] }.into(),
        BulkChunk { id: 1, offset: 1024, bytes: Bytes::new(vec![0xAB; 1024]).unwrap() }.into(),
        BulkEnd { id: 1 }.into(),
    ];
    out.extend(events());
    out
}

fn events() -> Vec<Message> {
    vec![
        RaceStart { head: EventHead { target_tick: 2000, ..head(1) }, order: List::new(vec![4, 9, 1]).unwrap() }.into(),
        Checkpoint { head: head(2), entity_id: 4, index: 7 }.into(),
        LapComplete { head: head(3), entity_id: 4, lap: 2, time_ms: 61_234 }.into(),
        RaceFinish { head: head(4), entity_id: 4, place: 1, time_ms: 183_000 }.into(),
        RaceResults {
            head: head(5),
            rows: List::new(vec![ResultRow {
                slot: 0,
                entity_id: 4,
                place: 1,
                time_ms: 183_000,
                best_lap_ms: 60_100,
                flags: 0,
            }])
            .unwrap(),
        }
        .into(),
        PursuitStart { head: head(6), pursuit_id: 2, targets: List::new(vec![4, 9]).unwrap(), heat: 3 }.into(),
        PursuitEnd { head: head(7), pursuit_id: 2, reason: PursuitEndReason::Evaded, entity_id: 4 }.into(),
        HeatChange { head: head(8), entity_id: 4, level: 4 }.into(),
        Infraction { head: head(9), entity_id: 4, kind: 2, fine: 750 }.into(),
        BustedProgress { head: head(10), entity_id: 4, fraction: 128 }.into(),
        RoadblockEvent {
            head: head(11),
            id: 300,
            state: RoadblockState::Standing,
            position: [1.0, -2.5, 3.0],
            yaw: 1.5,
        }
        .into(),
        SpikeEvent { head: head(12), id: 301, state: RoadblockState::Dodged, wheel: Some(3) }.into(),
        Collision {
            head: head(13),
            a: 4,
            b: None,
            impulse: 12_000.0,
            point: [10.0, 20.0, 0.5],
            normal: [0.0, 1.0, 0.0],
        }
        .into(),
        Damage { head: head(14), entity_id: 4, zone: 2, severity: 3, glazed: true }.into(),
        Puncture { head: head(15), entity_id: 4, wheel: 1 }.into(),
        Repair { head: head(16), entity_id: 4 }.into(),
        Respawn { head: head(17), entity_id: 4, position: [0.0; 3], yaw: -3.0, cause: RespawnCause::Reset }.into(),
        PhaseChange { head: head(18), phase: SessionPhase::Countdown }.into(),
        EntityRemoved { head: head(19), entity_id: 77, reason: RemoveReason::OutOfRange }.into(),
        PlayerJoined { head: head(20), slot: 6, player_id: [6; 16], name: Name::new("Ronnie").unwrap() }.into(),
        PlayerLeft { head: head(21), slot: 6, reason: LeaveReason::Timeout }.into(),
        RoleChanged { head: head(22), slot: 6, role: Role::Cop }.into(),
        ClockEvent { head: head(23), time_of_day: 600, rate: 1.0 }.into(),
    ]
}
