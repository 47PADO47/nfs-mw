use super::*;
use crate::loadout::CarRestriction;
use crate::message::{CollisionMode, DamageMode, JoinInProgress, List, Mode, SessionPhase, SettingsPatch};
use crate::roles::Rank;
use crate::version::{ModEntry, ModKind, ModList};

fn lan() -> Settings {
    defaults(ServerName::new("Rockport").unwrap())
}

/// A change that breaks one field.
type Breaks = fn(&mut Settings);

fn host_in(phase: SessionPhase) -> PatchContext {
    PatchContext { rank: Rank::Host, phase, racers: 2, spectators: 0, has_password: false }
}

#[test]
fn defaults_are_valid_and_sized_for_lan_and_dedicated() {
    let lan = lan();
    assert_eq!(validate(&lan), Ok(()));
    assert_eq!(lan.max_players, LAN_PLAYERS);
    let dedicated = dedicated_defaults(ServerName::new("Box").unwrap());
    assert_eq!(validate(&dedicated), Ok(()));
    assert_eq!((dedicated.max_players, dedicated.max_spectators), (DEDICATED_PLAYERS, DEDICATED_SPECTATORS));
    assert!(!lan.password_required && !lan.public && lan.nitrous && lan.collision == CollisionMode::Full);
}

#[test]
fn validate_names_the_broken_field() {
    let cases: [(Breaks, Field); 8] = [
        (|s| s.max_players = 17, Field::MaxPlayers),
        (|s| s.max_players = 1, Field::MaxPlayers),
        (|s| s.max_spectators = 9, Field::MaxSpectators),
        (|s| s.traffic_density = 101, Field::TrafficDensity),
        (|s| s.mode = Mode::Circuit, Field::Laps),
        (|s| s.cop_level_cap = 0, Field::CopLevelCap),
        (|s| s.mode = Mode::Drag, Field::Mode),
        (|s| s.allowed_cars = CarRestriction::Cars(List::empty()), Field::AllowedCars),
    ];
    for (breaks, field) in cases {
        let mut settings = lan();
        breaks(&mut settings);
        assert_eq!(validate(&settings), Err(SettingsError::Invalid(field)), "{field:?}");
    }
}

#[test]
fn a_custom_mode_needs_a_sim_mod() {
    let mut settings = lan();
    settings.mode = Mode::Custom(200);
    assert_eq!(validate(&settings), Err(SettingsError::Invalid(Field::Mode)));
    let entry = ModEntry::new("tag-plus", "1.0.0", [1; 32], ModKind::Code, false).unwrap();
    settings.required_mods = ModList::new(vec![entry]).unwrap();
    assert_eq!(validate(&settings), Ok(()));
}

#[test]
fn the_host_changes_settings_and_learns_what_changed() {
    let patch = SettingsPatch {
        mode: Some(Mode::Circuit),
        laps: Some(3),
        nitrous: Some(false),
        collision: Some(CollisionMode::Ghost),
        damage: Some(DamageMode::Off),
        pursuits: Some(true),
        ..SettingsPatch::default()
    };
    let (next, changed) = apply_patch(&lan(), &patch, &host_in(SessionPhase::Lobby)).unwrap();
    assert_eq!((next.mode, next.laps, next.nitrous), (Mode::Circuit, 3, false));
    assert_eq!(changed, vec![Field::Mode, Field::Laps, Field::Collision, Field::Nitrous, Field::Damage]);
}

#[test]
fn a_player_or_spoofed_patch_is_not_permitted() {
    let patch = SettingsPatch { max_players: Some(16), ..SettingsPatch::default() };
    let context = PatchContext { rank: Rank::Player, ..host_in(SessionPhase::Lobby) };
    assert_eq!(apply_patch(&lan(), &patch, &context), Err(SettingsError::NotPermitted));
    let admin = PatchContext { rank: Rank::Admin, ..context };
    assert!(apply_patch(&lan(), &patch, &admin).is_ok());
}

#[test]
fn race_fields_are_locked_while_a_race_runs() {
    let running = host_in(SessionPhase::Running);
    let track = SettingsPatch { track: Some(4), ..SettingsPatch::default() };
    assert_eq!(apply_patch(&lan(), &track, &running), Err(SettingsError::Locked(Field::Track)));
    let name = SettingsPatch {
        name: Some(ServerName::new("Night").unwrap()),
        join_in_progress: Some(JoinInProgress::Closed),
        ..SettingsPatch::default()
    };
    let (next, changed) = apply_patch(&lan(), &name, &running).unwrap();
    assert_eq!(next.name.as_str(), "Night");
    assert_eq!(changed, vec![Field::Name, Field::JoinInProgress]);
    assert!(apply_patch(&lan(), &track, &host_in(SessionPhase::Results)).is_ok());
}

#[test]
fn limits_cannot_drop_below_the_head_count_and_a_password_must_exist() {
    let context = PatchContext { racers: 6, spectators: 3, ..host_in(SessionPhase::Lobby) };
    let players = SettingsPatch { max_players: Some(4), ..SettingsPatch::default() };
    assert_eq!(apply_patch(&lan(), &players, &context), Err(SettingsError::BelowCount(Field::MaxPlayers)));
    let spectators = SettingsPatch { max_spectators: Some(2), ..SettingsPatch::default() };
    assert_eq!(apply_patch(&lan(), &spectators, &context), Err(SettingsError::BelowCount(Field::MaxSpectators)));
    let password = SettingsPatch { password_required: Some(true), ..SettingsPatch::default() };
    assert_eq!(apply_patch(&lan(), &password, &context), Err(SettingsError::NoPassword));
    let with = PatchContext { has_password: true, ..context };
    assert!(apply_patch(&lan(), &password, &with).unwrap().0.password_required);
}

#[test]
fn a_patch_that_breaks_a_rule_changes_nothing() {
    let patch = SettingsPatch { mode: Some(Mode::Knockout), ..SettingsPatch::default() };
    assert_eq!(apply_patch(&lan(), &patch, &host_in(SessionPhase::Lobby)), Err(SettingsError::Invalid(Field::Laps)));
}

#[test]
fn pairs_round_trip_every_field() {
    let mut settings = lan();
    settings.mode = Mode::Circuit;
    settings.laps = 4;
    settings.collision = CollisionMode::GhostStart;
    settings.allowed_cars = CarRestriction::Cars(List::new(vec![7, 9]).unwrap());
    settings.join_in_progress = JoinInProgress::BetweenRaces;
    let pairs = to_pairs(&settings);
    assert_eq!(pairs.len(), Field::ALL.len());
    assert!(pairs.contains(&("allowed_cars", "cars:7,9".to_owned())));
    let borrowed: Vec<(&str, &str)> = pairs.iter().map(|(key, value)| (*key, value.as_str())).collect();
    assert_eq!(from_pairs(lan(), borrowed), Ok(settings));
}

#[test]
fn pairs_parse_each_value_kind() {
    let pairs = [
        ("mode", "custom:200"),
        ("allowed_cars", "class:3"),
        ("damage", "off"),
        ("max_players", "12"),
        ("public", "true"),
    ];
    let mut settings = lan();
    for (key, text) in pairs {
        set(&mut settings, key, text).unwrap();
    }
    assert_eq!(settings.mode, Mode::Custom(200));
    assert_eq!(settings.allowed_cars, CarRestriction::Class(3));
    assert_eq!((settings.max_players, settings.public, settings.damage), (12, true, DamageMode::Off));
    set(&mut settings, "allowed_cars", "cap:450").unwrap();
    assert_eq!(settings.allowed_cars, CarRestriction::PerformanceCap(450));
}

#[test]
fn bad_pairs_are_refused() {
    let mut settings = lan();
    assert_eq!(set(&mut settings, "colour", "red"), Err(SettingsError::UnknownKey));
    assert_eq!(set(&mut settings, "mode", "custom:12"), Err(SettingsError::Invalid(Field::Mode)));
    assert_eq!(set(&mut settings, "laps", "-1"), Err(SettingsError::Invalid(Field::Laps)));
    assert_eq!(set(&mut settings, "allowed_cars", "cars:"), Err(SettingsError::Invalid(Field::AllowedCars)));
    assert_eq!(set(&mut settings, "name", "  "), Err(SettingsError::Invalid(Field::Name)));
    assert_eq!(from_pairs(lan(), [("max_players", "40")]), Err(SettingsError::Invalid(Field::MaxPlayers)));
}
