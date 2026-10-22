use super::*;
use crate::message::{Reader, Wire, Writer};

fn build(minor: u16) -> BuildInfo {
    BuildInfo {
        proto: ProtoVersion { major: PROTO_MAJOR, minor },
        sim_build: SimBuild::from_parts("abc123", "x86_64-pc-windows-msvc"),
        data_id: DataId::compute(&[7; 32], &[("GLOBAL/GLOBALB.BUN", 100), ("TRACKS/L2RA.BUN", 200)]),
    }
}

fn offer<'a>(info: &BuildInfo, mods: &'a [ModEntry]) -> ClientOffer<'a> {
    ClientOffer { proto: info.proto, sim_build: info.sim_build, data_id: info.data_id, mods }
}

fn entry(id: &str, version: &str, hash: u8, client_only: bool) -> ModEntry {
    ModEntry::new(id, version, [hash; 32], ModKind::Code, client_only).unwrap()
}

#[test]
fn same_build_is_admitted_at_its_minor() {
    let server = build(3);
    let admitted = check_join(&server, &ServerPolicy::default(), &[], &offer(&server, &[])).unwrap();
    assert_eq!(admitted, Admitted { minor: 3, sim_build_matches: true });
}

#[test]
fn major_mismatch_names_the_side_that_must_update() {
    let server = build(0);
    let mut client = offer(&server, &[]);
    client.proto.major = PROTO_MAJOR - 1;
    let refusal = check_join(&server, &ServerPolicy::default(), &[], &client).unwrap_err();
    assert_eq!(refusal, Refusal { reason: RejectReason::ProtocolMismatch, detail: DETAIL_CLIENT_MUST_UPDATE });
    client.proto.major = PROTO_MAJOR + 1;
    let refusal = check_join(&server, &ServerPolicy::default(), &[], &client).unwrap_err();
    assert_eq!(refusal.detail, DETAIL_SERVER_MUST_UPDATE);
}

#[test]
fn newer_client_falls_back_to_the_server_minor() {
    let server = build(2);
    let mut client = offer(&server, &[]);
    client.proto.minor = 5;
    let admitted = check_join(&server, &ServerPolicy::default(), &[], &client).unwrap();
    assert_eq!(admitted.minor, 2);
    assert_eq!(negotiate_minor(client.proto, server.proto), Some(2));
}

#[test]
fn older_client_below_the_served_range_is_refused() {
    let server = build(4);
    let policy = ServerPolicy { min_minor: 3, ..ServerPolicy::default() };
    let mut client = offer(&server, &[]);
    client.proto.minor = 3;
    assert_eq!(check_join(&server, &policy, &[], &client).unwrap().minor, 3);
    client.proto.minor = 2;
    let refusal = check_join(&server, &policy, &[], &client).unwrap_err();
    assert_eq!(refusal, Refusal { reason: RejectReason::ProtocolMismatch, detail: DETAIL_CLIENT_MUST_UPDATE });
}

#[test]
fn min_minor_above_the_server_minor_is_clamped() {
    let server = build(1);
    let policy = ServerPolicy { min_minor: 9, ..ServerPolicy::default() };
    assert!(check_join(&server, &policy, &[], &offer(&server, &[])).is_ok());
}

#[test]
fn other_game_data_is_refused() {
    let server = build(0);
    let mut client = offer(&server, &[]);
    client.data_id = DataId::compute(&[8; 32], &[]);
    let refusal = check_join(&server, &ServerPolicy::default(), &[], &client).unwrap_err();
    assert_eq!(refusal.reason, RejectReason::DataMismatch);
}

#[test]
fn sim_build_only_matters_when_required() {
    let server = build(0);
    let mut client = offer(&server, &[]);
    client.sim_build = SimBuild::from_parts("def456", "x86_64-unknown-linux-gnu");
    let admitted = check_join(&server, &ServerPolicy::default(), &[], &client).unwrap();
    assert!(!admitted.sim_build_matches);
    let strict = ServerPolicy { require_sim_build: true, ..ServerPolicy::default() };
    assert_eq!(check_join(&server, &strict, &[], &client).unwrap_err().reason, RejectReason::SimBuildMismatch);
}

#[test]
fn data_id_ignores_file_order_and_sees_sizes() {
    let a = DataId::compute(&[1; 32], &[("A", 1), ("B", 2)]);
    let b = DataId::compute(&[1; 32], &[("B", 2), ("A", 1)]);
    let c = DataId::compute(&[1; 32], &[("A", 1), ("B", 3)]);
    assert_eq!(a, b);
    assert_ne!(a, c);
    assert_eq!(a.short(), [a.bytes[0], a.bytes[1], a.bytes[2], a.bytes[3]]);
}

#[test]
fn mods_lenient_ignores_extras_and_client_only() {
    let required = [entry("speed.tweaks", "1.0.0", 1, false)];
    let offered = [entry("speed.tweaks", "1.0.0", 1, false), entry("extra.mod", "2.0.0", 2, false)];
    assert!(compare_mods(&required, &offered, ModPolicy::Lenient).is_ok());
    let diff = compare_mods(&required, &offered, ModPolicy::Strict).unwrap_err();
    assert_eq!(diff.extra, vec!["extra.mod".to_owned()]);
    let hud = [entry("speed.tweaks", "1.0.0", 1, false), entry("hud.skin", "1.0.0", 3, true)];
    assert!(compare_mods(&required, &hud, ModPolicy::Strict).is_ok());
}

#[test]
fn mods_report_missing_and_different() {
    let required = [entry("a.b", "1.2.0", 1, false), entry("c.d", "1.0.0", 2, false)];
    let offered = [entry("a.b", "1.3.0", 1, false)];
    let diff = compare_mods(&required, &offered, ModPolicy::Lenient).unwrap_err();
    assert_eq!(diff.missing, vec!["c.d".to_owned()]);
    assert_eq!(diff.different, vec!["a.b".to_owned()]);
    assert_eq!(diff.detail(), 2);
    let server = build(0);
    let refusal = check_join(&server, &ServerPolicy::default(), &required, &offer(&server, &offered)).unwrap_err();
    assert_eq!(refusal, Refusal { reason: RejectReason::ModMismatch, detail: 2 });
}

#[test]
fn mods_off_refuses_any_sim_mod() {
    let offered = [entry("a.b", "1.0.0", 1, false)];
    assert!(compare_mods(&[], &offered, ModPolicy::Off).is_err());
    let hud = [entry("hud.skin", "1.0.0", 1, true)];
    assert!(compare_mods(&[], &hud, ModPolicy::Off).is_ok());
}

#[test]
fn mod_ids_follow_the_charset() {
    assert!(is_mod_id("my_mod-2.0"));
    assert!(!is_mod_id("My.Mod"));
    assert!(!is_mod_id(""));
    assert!(!is_mod_id(&"a".repeat(65)));
    assert!(ModEntry::new("bad id", "1", [0; 32], ModKind::Content, false).is_err());
}

#[test]
fn mod_entry_round_trips_and_refuses_a_bad_id() {
    let original = entry("speed.tweaks", "1.0.0", 9, false);
    let mut bytes = Vec::new();
    original.put(&mut Writer::new(&mut bytes));
    assert_eq!(ModEntry::take(&mut Reader::new(&bytes)).unwrap(), original);
    let at = bytes.iter().position(|b| *b == b's').unwrap();
    bytes[at] = b'S';
    assert!(ModEntry::take(&mut Reader::new(&bytes)).is_err());
}

#[test]
fn discovery_labels() {
    let local = build(0);
    assert_eq!(classify_server(&local, PROTO_MAJOR, local.data_id.short()), ServerLabel::Compatible);
    assert_eq!(classify_server(&local, PROTO_MAJOR, [0; 4]), ServerLabel::DataMismatch);
    assert_eq!(
        classify_server(&local, PROTO_MAJOR + 1, local.data_id.short()),
        ServerLabel::VersionMismatch { detail: DETAIL_CLIENT_MUST_UPDATE }
    );
}

#[test]
fn features_wait_for_a_minor_that_defines_them() {
    assert!(!Feature::Voice.enabled_at(u16::MAX));
    assert!(!Feature::SpectatorHints.enabled_at(PROTO_MINOR));
    assert_eq!(noise_prologue(1), "nfsmw-mp/1");
    assert_eq!(HandshakeReject::from_code(2), Some(HandshakeReject::ProtocolMajor));
    assert_eq!(HandshakeReject::from_code(0), None);
}
