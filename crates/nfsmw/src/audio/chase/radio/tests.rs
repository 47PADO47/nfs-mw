//! The chase radio's adapter: tables, parameter mapping and the snapshot, without a game.

use blackbox_pursuit::speech::{self as m, Line, Notice};
use blackbox_pursuit::{Infraction, InfractionLog};

use super::params::event_name;
use super::snapshot::{Prev, notices};
use super::speech_params;
use super::tables::{self, Place};
use crate::audio::speech::params::{Param, signature, values};
use crate::audio::{CopFeed, PursuitFeed};

/// Every parameter set to a value that is valid for it.
fn full() -> m::Params {
    let battalion = tables::CITY;
    m::Params {
        pursuit_type: Some(m::PursuitType::Reckless),
        suspects: Some(m::Suspects::Multiple),
        battalion: Some(battalion),
        call_sign: Some(7),
        subject: Some(m::Unit { battalion: tables::RHINO, call_sign: 3 }),
        intensity: Some(m::Intensity::High),
        yes: Some(true),
        lost_for: Some(m::LostFor::ForAWhile),
        car: Some(m::Car { kind: values::car_type::PORSCHE, colour: values::car_color::RED }),
        speed: Some(150.0),
        measurement: Some(m::Measurement::Imperial),
        direction: Some(m::Direction::East),
        road: Some(5),
        encounter: Some(m::Encounter::Subsequent),
        backup: Some(m::Backup::Rhino),
        eta: Some(m::Eta::Min1),
        strategy: Some(m::SelfStrategy::Ram),
        rolling: Some(m::RollingStrategy::Box),
        position: Some(m::Position::Left),
        new_strategy: Some(true),
        world_object: Some(m::WorldObject::Semi),
        units: Some(m::Units::Multiple),
        bailout: Some(m::Bailout::Damage),
        about_other: Some(true),
        ev: Some(m::Ev::GasStationFire),
        offroad: Some((3, true)),
        heat_level: Some(4),
        jurisdiction: Some(m::Jurisdiction::Federal),
        roadblock: Some(m::RoadblockKind::Spikes),
        engage: Some(m::Engage::Spikes),
        spike_position: Some(m::SpikePosition::Center),
        heli_strategy: Some(m::HeliStrategy::Roadblock),
        heli_lost: Some(m::HeliLost::InTunnel),
        heli_bailout: Some(m::HeliBailout::FuelLow),
        hazard: Some(m::Hazard::ApproachingTunnel),
        address_of: Some(battalion),
        generic_address: Some(false),
    }
}

#[test]
fn every_line_names_an_event_with_an_argument_list() {
    for &line in Line::ALL {
        assert!(signature(event_name(line)).is_some(), "{line:?}: {} has no argument list", event_name(line));
    }
}

#[test]
fn a_line_has_a_speaker_argument_unless_it_is_voiceless() {
    for &line in Line::ALL {
        let (has_speaker, _) = signature(event_name(line)).unwrap();
        assert_eq!(has_speaker, !line.is_voiceless(), "{line:?}");
    }
}

#[test]
fn the_event_names_are_distinct() {
    let mut names: Vec<&str> = Line::ALL.iter().map(|l| event_name(*l)).collect();
    names.sort_unstable();
    let all = names.len();
    names.dedup();
    assert_eq!(names.len(), all, "two lines share an event");
}

#[test]
fn every_parameter_set_lies_within_the_choices_of_its_argument() {
    for &line in Line::ALL {
        let params = speech_params(line, &full());
        let (_, fields) = signature(event_name(line)).unwrap();
        for (param, value) in params.iter() {
            assert!(fields.contains(&param), "{line:?} sets {} which it does not take", param.name());
            assert!(value != 0 && value & !param.choices() == 0, "{line:?}: {} = {value:#x}", param.name());
        }
    }
}

#[test]
fn a_line_gets_every_parameter_of_its_argument_list_that_the_machine_names() {
    // The ones the machine does not name are the ten-code and the region, which the adapter fills itself.
    for &line in Line::ALL {
        let params = speech_params(line, &full());
        let (_, fields) = signature(event_name(line)).unwrap();
        for &field in fields {
            assert!(params.get(field).is_some(), "{line:?} leaves {} unset", field.name());
        }
    }
}

#[test]
fn the_vehicle_stop_names_its_unit_in_the_games_values() {
    let params = speech_params(Line::AttemptVehicleStop, &full());
    assert_eq!(params.get(Param::SpeakerBattalion), Some(values::speaker_battalion::CITY));
    assert_eq!(params.get(Param::SpeakerCallSignId), Some(1 << 6));
    assert_eq!(params.get(Param::PursuitType), Some(values::pursuit_type::RECKLESS));
    assert_eq!(params.get(Param::NumSuspects), Some(values::num_suspects::MULTIPLE_SUSPECTS));
}

#[test]
fn the_vehicle_report_speed_is_the_step_of_the_players_unit() {
    let params = speech_params(Line::VehicleReport, &full());
    assert_eq!(params.get(Param::Speed), Some(values::speed::SPEEDS_IN_EXCESS_OF_140));
    assert_eq!(params.get(Param::CarColor), Some(values::car_color::RED));
    assert_eq!(params.get(Param::Measurement), Some(values::measurement::IMPERIAL_ONLY));
}

#[test]
fn the_location_comes_from_the_road_name_table() {
    let params = speech_params(Line::LocationReport, &full());
    assert_eq!(params.get(Param::LocationRegion), Some(values::location_region::COLLEGE_TOWN));
    assert_eq!(params.get(Param::Location), Some(1 << 4));
    assert_eq!(params.get(Param::Direction), Some(values::direction::EASTBOUND));
}

#[test]
fn a_road_with_no_name_leaves_the_place_open() {
    let mut p = full();
    p.road = Some(84);
    let params = speech_params(Line::LocationReport, &p);
    assert_eq!(params.get(Param::Location), None);
    assert_eq!(params.get(Param::LocationRegion), None);
}

#[test]
fn the_backup_request_and_dispatchs_answer_use_their_own_values() {
    let asked = speech_params(Line::CallForBackup, &full());
    assert_eq!(asked.get(Param::BackupType), Some(values::backup_type::RHINO_UNIT));
    assert_eq!(asked.get(Param::Code), Some(values::code::DONT_USE_10_CODE));
    let answered = speech_params(Line::DispBackupReply, &full());
    assert_eq!(answered.get(Param::DispBackupType), Some(values::disp_backup_type::RHINO_UNIT));
}

#[test]
fn the_escalation_and_the_911_report_name_an_address_group() {
    let escalation = speech_params(Line::DispPursuitEscalation, &full());
    assert_eq!(escalation.get(Param::AddressGroupType), Some(values::address_group_type::CITY));
    let mut p = full();
    p.address_of = None;
    p.generic_address = Some(true);
    let report = speech_params(Line::Disp911Report, &p);
    assert_eq!(report.get(Param::AddressGroupType), Some(values::address_group_type::GENERIC_ANY));
    p.generic_address = Some(false);
    let report = speech_params(Line::Disp911Report, &p);
    assert_eq!(
        report.get(Param::AddressGroupType),
        Some(values::address_group_type::COLLEGE_TOWN),
        "road 5 is college town"
    );
}

#[test]
fn the_road_name_table_covers_the_named_roads_and_no_others() {
    let named: Vec<u16> = (0..=120).filter(|&r| tables::place(r).is_some()).collect();
    let mut expected: Vec<u16> = (1..=83).collect();
    expected.extend([85, 86]);
    assert_eq!(named, expected, "road 84 and 87 and the off-road places have no name");
    for road in named {
        let Place { region, location } = tables::place(road).unwrap();
        assert!((1..=31).contains(&location), "road {road}: location {location}");
        assert!(region & values::location_region::COLLEGE_TOWN != 0 || region <= 8, "road {road}: region {region}");
    }
}

#[test]
fn road_names_pick_the_battalion_of_their_area() {
    assert_eq!(tables::road_battalion(5), Some(tables::ROSEWOOD));
    assert_eq!(tables::road_battalion(40), Some(tables::COASTAL));
    assert_eq!(tables::road_battalion(62), Some(tables::COASTAL));
    assert_eq!(tables::road_battalion(70), Some(tables::CITY));
    assert_eq!(tables::road_battalion(0), None);
    assert_eq!(tables::road_battalion(88), None);
}

#[test]
fn the_cop_models_pick_their_battalion() {
    use crate::audio::Vehicle;
    assert_eq!(tables::vehicle_of("COPSUV"), Vehicle::Suv);
    assert_eq!(tables::vehicle_of("copsportghost"), Vehicle::SuperPursuit);
    assert_eq!(tables::vehicle_of("copcross"), Vehicle::Cross);
    assert_eq!(tables::vehicle_of("copmidsize"), Vehicle::Patrol);
    assert_eq!(tables::vehicle_battalion(Vehicle::Suv), Some(tables::RHINO));
    assert_eq!(tables::vehicle_battalion(Vehicle::Patrol), None);
}

#[test]
fn the_voice_layout_is_the_speech_interfaces() {
    let layout = tables::layout();
    assert_eq!((layout.dispatch, layout.heli, layout.cross), (1, 2, 9));
    assert_eq!(layout.cop_voices().len(), 7);
    assert_eq!(layout.pool_size(tables::CITY), Some(20));
    assert_eq!(layout.pool_size(tables::SUPER_PURSUIT), Some(5));
}

fn feed(edit: impl FnOnce(&mut PursuitFeed)) -> PursuitFeed {
    let mut f = PursuitFeed { heat: 1, intensity: 1.0, secs: 3.0, secs_to_evade: 30.0, ..Default::default() };
    edit(&mut f);
    f
}

fn log_of(infractions: &[Infraction]) -> InfractionLog {
    let mut log = InfractionLog::default();
    let fines = blackbox_pursuit::Fines::new([0; 8]);
    for &i in infractions {
        log.record(i, &fines);
    }
    log
}

#[test]
fn a_chase_that_starts_with_an_infraction_hands_it_to_the_machine() {
    let mut prev = Prev::default();
    let first = notices(&mut prev, &feed(|f| f.speech.status.infractions = log_of(&[Infraction::Speeding])));
    assert_eq!(first, vec![Notice::Infraction(Infraction::Speeding)]);
    let again = notices(&mut prev, &feed(|f| f.speech.status.infractions = log_of(&[Infraction::Speeding])));
    assert!(again.is_empty());
}

#[test]
fn a_new_infraction_is_a_notice_and_an_assault_is_a_hit_on_the_nearest_cop() {
    let mut prev = Prev::default();
    notices(&mut prev, &feed(|_| {}));
    let after = feed(|f| {
        f.speech.status.infractions = log_of(&[Infraction::Assault]);
        f.speech.push_cop(CopFeed { id: 7, active: true, distance: 50.0, ..CopFeed::default() });
        f.speech.push_cop(CopFeed { id: 8, active: true, distance: 20.0, ..CopFeed::default() });
    });
    let got = notices(&mut prev, &after);
    assert_eq!(got[0], Notice::Infraction(Infraction::Assault));
    assert!(matches!(got[1], Notice::PlayerHitCop { cop: 8, deliberate: true, .. }), "{got:?}");
}

#[test]
fn the_player_hitting_traffic_is_a_notice_on_the_rising_edge_only() {
    let mut prev = Prev::default();
    assert!(notices(&mut prev, &feed(|f| f.player_hit = true)).is_empty(), "the first frame only records");
    assert!(notices(&mut prev, &feed(|f| f.player_hit = true)).is_empty());
    assert!(notices(&mut prev, &feed(|_| {})).is_empty());
    let hit = notices(&mut prev, &feed(|f| f.player_hit = true));
    assert!(matches!(hit.as_slice(), [Notice::PlayerHitTraffic { .. }]), "{hit:?}");
}

#[test]
fn the_snapshot_carries_the_cops_and_the_players_car() {
    let level = nfsmw_data::speech_chase::LevelSpeech { roadblocks: true, spikes: true, ..Default::default() };
    let f = feed(|f| {
        f.player_mph = 90.0;
        f.speech.player.car_type = values::car_type::PORSCHE;
        f.speech.player.car_colour = values::car_color::RED;
        f.speech.player.road = Some(12);
        f.speech.push_cop(CopFeed { id: 3, active: true, los: true, road: 40, ..CopFeed::default() });
        f.rain_intensity = 0.8;
    });
    let snap = super::snapshot::snapshot(&f, &level);
    assert!(snap.pursuit.exists && snap.pursuit.perp_in_sight);
    assert_eq!(snap.player.car, Some(m::Car { kind: values::car_type::PORSCHE, colour: values::car_color::RED }));
    assert!(snap.player.commentable);
    assert_eq!(snap.player.road, Some(12));
    assert_eq!(snap.cops.len(), 1);
    assert_eq!(snap.cops[0].road_battalion, Some(tables::COASTAL));
    assert!(snap.level.roadblocks && snap.level.spikes && !snap.level.heli_roadblocks);
    assert!(snap.weather);
}

#[test]
fn a_car_nobody_recorded_is_not_described() {
    let f = feed(|f| {
        f.speech.player.car_type = values::car_type::PAGANI;
        f.speech.player.car_colour = values::car_color::RED;
    });
    let snap = super::snapshot::snapshot(&f, &Default::default());
    assert!(snap.player.car.is_none() && !snap.player.commentable);
}
