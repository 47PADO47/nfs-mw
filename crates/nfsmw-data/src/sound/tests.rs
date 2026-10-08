//! The pure parts of the sound module; the database reads are checked against the install in
//! `tests/real_install/sound.rs`.

use super::*;

#[test]
fn stock_cars_take_the_first_engine() {
    for current in 0..=4 {
        assert_eq!(audio_engine_level(0, current, 1), 0);
        assert_eq!(audio_engine_level(0, current, 3), 0);
    }
}

#[test]
fn engine_level_follows_the_installed_upgrade() {
    // Four upgrade levels: base 0, so levels 1 to 2 map to entry 1 and 3 to 4 to entry 2.
    let levels: Vec<usize> = (0..=4).map(|cur| audio_engine_level(4, cur, 3)).collect();
    assert_eq!(levels, [0, 1, 1, 2, 2]);
    // Three upgrade levels: base 1, the second entry from the second installed level.
    let levels: Vec<usize> = (0..=3).map(|cur| audio_engine_level(3, cur, 3)).collect();
    assert_eq!(levels, [0, 0, 1, 1]);
    // Two levels: base 2, only the last level reaches the second entry.
    let levels: Vec<usize> = (0..=2).map(|cur| audio_engine_level(2, cur, 3)).collect();
    assert_eq!(levels, [0, 1, 1]);
    // One level: base 3, never past the first entry.
    assert_eq!(audio_engine_level(1, 4, 3), 0);
}

#[test]
fn engine_level_never_leaves_the_array() {
    assert_eq!(audio_engine_level(4, 4, 1), 0);
    assert_eq!(audio_engine_level(4, 4, 2), 1);
    assert_eq!(audio_engine_level(4, 4, 0), 0);
    assert_eq!(audio_engine_level(-3, 99, 5), 0, "nonsense counts read as no upgrades");
}

#[test]
fn shift_and_turbo_entries_pick_the_last_level_reached() {
    let levels = [0, 1, 2, 3, 4];
    // Four upgrades: the installed level 0 is `4 - 4 + 0 = 0`.
    assert_eq!(upgrade_entry(&levels, 4, 0), 0);
    assert_eq!(upgrade_entry(&levels, 4, 2), 2);
    assert_eq!(upgrade_entry(&levels, 4, 4), 4);
    // Three upgrades start at level 1.
    assert_eq!(upgrade_entry(&levels, 3, 0), 1);
    // Entries that only start later do not apply to a stock car.
    assert_eq!(upgrade_entry(&[0, 2, 3, 4], 3, 0), 0);
    assert_eq!(upgrade_entry(&[0, 2, 3, 4], 3, 1), 1);
    assert_eq!(upgrade_entry(&[], 3, 1), 0);
}

fn tvr_cerbera() -> EngineSound {
    EngineSound {
        name: "tvr_cerb".into(),
        accel_loop: "GIN_TVR_Cerbera.gin".into(),
        decel_loop: "GIN_TVR_Cerbera_DCL.gin".into(),
        bank_main: "CAR_66_ENG_MB_EE.abk".into(),
        banks_aux: vec!["CAR_66_ENG_MB_SPU.abk".into()],
        sweet_banks: vec!["SWTN_CAR_66_MB.abk".into(), "CAR_WHINE_00.abk".into()],
        car_id: 66,
        group: EngineGroup::V8,
        priority: 5.0,
        may_upgrade_to_v8: false,
        has_transmission_loop: true,
        min_rpm: 1500.0,
        max_rpm: 7784.0,
        rpm_map: [0.0, 0.4136, 0.6893, 1.0],
        master_volume: 23500,
        mix: EngineMix::default(),
        decel_window: DecelWindow {
            min_rpm: 2079.0,
            max_rpm: 7570.0,
            fade_in_fraction: 0.25,
            fade_out_fraction: 0.001,
        },
        low_pass_cutoff: 24000,
        shift_sweet_volume: 25000,
        sputter_volume: 32000,
        decel_pitch_offset: 0.0,
        accel_transition: None,
    }
}

#[test]
fn engine_rpm_maps_onto_the_ginsu_range() {
    let e = tvr_cerbera();
    assert_eq!(e.ginsu_frequency(1000.0), 1500.0);
    assert_eq!(e.ginsu_frequency(10000.0), 7784.0);
    assert_eq!(e.ginsu_frequency(-5.0), 1500.0, "clamped below the sound scale");
    assert_eq!(e.ginsu_frequency(99_999.0), 7784.0, "clamped above it");
    assert!((e.ginsu_frequency(5500.0) - (1500.0 + 0.5 * 6284.0)).abs() < 0.01);
}

#[test]
fn the_rpm_remap_runs_through_its_end_points() {
    let e = tvr_cerbera();
    assert_eq!(e.remap_rpm(0.0), 0.0);
    assert_eq!(e.remap_rpm(1.0), 1.0);
    assert_eq!(e.remap_rpm(-1.0), 0.0);
    assert_eq!(e.remap_rpm(2.0), 1.0);
    let mid = e.remap_rpm(0.5);
    // 0.125 * 0 + 0.375 * 0.4136 + 0.375 * 0.6893 + 0.125 * 1
    assert!((mid - 0.5386).abs() < 0.001, "{mid}");
    assert!(mid > 0.5, "the audio revs run ahead of the engine");
}

#[test]
fn the_decel_window_fades_in_holds_and_fades_out() {
    let w = tvr_cerbera().decel_window;
    assert_eq!(w.gain(1000.0), 0.0);
    assert_eq!(w.gain(2079.0), 0.0);
    assert_eq!(w.gain(7570.0), 0.0);
    assert_eq!(w.gain(9000.0), 0.0);
    let width = 7570.0 - 2079.0;
    let half_fade = 2079.0 + width * 0.125;
    assert!((w.gain(half_fade) - 0.5).abs() < 1e-3);
    assert_eq!(w.gain(2079.0 + width * 0.25), 1.0);
    assert_eq!(w.gain(5000.0), 1.0);
    let half_out = 7570.0 - width * 0.0005;
    assert!((w.gain(half_out) - 0.5).abs() < 1e-3);
    let nothing = DecelWindow::default();
    assert_eq!(nothing.gain(3000.0), 0.0);
}

#[test]
fn mix_levels_interpolate() {
    let m = MixLevels { steady: 1.0, large: 0.5 };
    assert_eq!(m.at(0.0), 1.0);
    assert_eq!(m.at(1.0), 0.5);
    assert_eq!(m.at(0.5), 0.75);
}

#[test]
fn engine_group_decodes_the_enum() {
    assert_eq!(EngineGroup::from_value(0), EngineGroup::V4);
    assert_eq!(EngineGroup::from_value(2), EngineGroup::V8);
    assert_eq!(EngineGroup::from_value(9), EngineGroup::Other(9));
}

#[test]
fn a_shift_stage_scales_its_curve_by_time_and_rpm() {
    // A curve from (0, 0) to (1, -1) with straight control points.
    let stage = ShiftStage {
        rpm: 3500,
        time_ms: 200,
        curve: [[0.0, 0.0], [1.0 / 3.0, -1.0 / 3.0], [2.0 / 3.0, -2.0 / 3.0], [1.0, -1.0]],
    };
    assert_eq!(stage.point(0.0), (0.0, 0.0));
    let (t, rpm) = stage.point(1.0);
    assert!((t - 200.0).abs() < 1e-3 && (rpm + 3500.0).abs() < 1e-2);
    let (t, rpm) = stage.point(0.5);
    assert!((t - 100.0).abs() < 1e-3 && (rpm + 1750.0).abs() < 1e-2);
}

fn shift(bank: &str) -> ShiftSound {
    ShiftSound {
        name: "0x6EB87040".into(),
        bank: bank.into(),
        up_sound_delay: 0.02,
        down_sound_delay: 0.05,
        up_volume: 32767,
        down_volume: 22000,
        up_engage_attack_volume: 0.15,
        up_engage_attack_ms: 500,
        up_disengage_fall: Vec::new(),
        up_engage: None,
        down_disengage_fall: (0, 0),
        down_engage_rise: (0, 0),
        down_engage_fall: (0, 0),
        down_reattach_scale: 0.0,
        lfo_rpm: Wobble::default(),
        lfo_volume: Wobble::default(),
    }
}

#[test]
fn a_cars_files_are_listed_with_their_folders() {
    let car = CarSound {
        car: "bmwm3gtr".into(),
        engine: tvr_cerbera(),
        shift: shift("GEAR_MED_Lev3.abk"),
        turbo: None,
        accel_transition: AccelTransition::default(),
        engine_level: 0,
        banks: GlobalBanks { skids: vec!["SKID_BIG_MB.abk".into()], nitrous: vec!["Nitrous_00_MB.abk".into()] },
        collision: CollisionSounds::default(),
    };
    assert_eq!(
        car.files(),
        [
            "SOUND/ENGINE/GIN_TVR_Cerbera.gin",
            "SOUND/ENGINE/GIN_TVR_Cerbera_DCL.gin",
            "SOUND/ENGINE/CAR_66_ENG_MB_EE.abk",
            "SOUND/ENGINE/CAR_66_ENG_MB_SPU.abk",
            "SOUND/ENGINE/SWTN_CAR_66_MB.abk",
            "SOUND/ENGINE/CAR_WHINE_00.abk",
            "SOUND/SHIFTING/GEAR_MED_Lev3.abk",
            "SOUND/NOS/Nitrous_00_MB.abk",
            "SOUND/SKIDS/SKID_BIG_MB.abk",
        ]
    );
    let with_turbo = CarSound {
        turbo: Some(TurboSound {
            name: "turbo_med_00".into(),
            bank: "TURBO_TUN_MED_0_MB.abk".into(),
            spool_volume: 11000,
            charge_time: 20.0,
            leak_rate: 0.5,
            blowoff_volume: [13000, 13000],
        }),
        ..car
    };
    assert!(with_turbo.files().contains(&"SOUND/TURBO/TURBO_TUN_MED_0_MB.abk".to_owned()));
}
