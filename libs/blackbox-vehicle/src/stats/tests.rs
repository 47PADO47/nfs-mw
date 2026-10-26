use glam::Vec3;

use super::*;
use crate::math::FT_LB_TO_NM;
use crate::performance::measure;

const BOX: Vec3 = Vec3::new(0.9, 0.55, 2.2);

/// A mid-engined sports car with the numbers of a real one: 1397 kg, 440 N m at 5600 rpm, seven gears.
fn sports_car() -> CarStats {
    CarStats {
        mass_kg: 1397.0,
        front_weight_pct: 38.4,
        cg_height_m: 0.45,
        wheelbase_m: 2.45,
        track_m: [1.54, 1.52],
        tire_radius_m: [0.34, 0.343],
        tire_width_m: [0.245, 0.295],
        idle_rpm: 1000.0,
        redline_rpm: 7800.0,
        torque_curve: vec![[1000.0, 360.0], [3300.0, 419.0], [5600.0, 440.0], [7800.0, 390.0]],
        gear_ratios: vec![3.91, 2.29, 1.55, 1.3, 1.08, 0.88, 0.71],
        reverse_ratio: 2.68,
        final_drive: 3.44,
        tire_grip: [1.5, 1.4],
        brake_torque_nm: [1300.0, 1350.0],
        top_speed: Some(84.5),
        ..CarStats::default()
    }
}

#[test]
fn the_generic_car_is_close_to_the_example_car() {
    let spec = VehicleSpec::from_stats(&CarStats::default(), Vec3::new(0.9, 0.62, 2.2));
    let example = VehicleSpec::example();
    assert_eq!(spec.mass, example.mass);
    assert_eq!(spec.transmission.gear_ratio, vec![3.4, 0.0, 3.6, 2.2, 1.5, 1.1, 0.9, 0.75]);
    assert!((spec.chassis.spring_stiffness[0] / example.chassis.spring_stiffness[0] - 1.0).abs() < 0.15);
    assert!((spec.chassis.shock_stiffness[0] / example.chassis.shock_stiffness[0] - 1.0).abs() < 0.15);
    let peak = |torque: &[f32]| torque.iter().copied().fold(0.0, f32::max);
    assert!((peak(&spec.engine.torque) - peak(&example.engine.torque)).abs() < 5.0, "{:?}", spec.engine.torque);
}

#[test]
fn the_tyre_size_makes_the_stats_radius() {
    let spec = VehicleSpec::from_stats(&sports_car(), BOX);
    assert!((spec.tires.radius(0) - 0.34).abs() < 1e-4, "{}", spec.tires.radius(0));
    assert!((spec.tires.radius(1) - 0.343).abs() < 1e-4, "{}", spec.tires.radius(1));
    assert!((spec.tires.section_width[1] - 295.0).abs() < 1e-3);
}

#[test]
fn the_engine_table_holds_the_stats_curve() {
    let stats = sports_car();
    let spec = VehicleSpec::from_stats(&stats, BOX);
    assert_eq!((spec.engine.idle, spec.engine.red_line, spec.engine.max_rpm), (1000.0, 7800.0, 7800.0));
    assert_eq!(spec.engine.torque.len(), SAMPLES);
    let peak = spec.engine.torque.iter().copied().fold(0.0, f32::max);
    assert!((peak * FT_LB_TO_NM - 440.0).abs() < 8.0, "{peak}");
    assert!((spec.engine.torque_nm(5600.0) - stats.torque_at(5600.0)).abs() < 6.0);
}

#[test]
fn the_gearbox_has_reverse_neutral_and_the_forward_gears() {
    let spec = VehicleSpec::from_stats(&sports_car(), BOX);
    let ratios = &spec.transmission.gear_ratio;
    assert_eq!(ratios.len(), 9);
    assert_eq!((ratios[0], ratios[1], ratios[2], ratios[8]), (2.68, 0.0, 3.91, 0.71));
    assert_eq!(spec.transmission.final_gear, 3.44);
    assert_eq!(spec.transmission.gear_efficiency.len(), 9);
}

#[test]
fn the_drive_sets_the_torque_split_and_the_differentials() {
    let mut stats = sports_car();
    let split = |stats: &CarStats| VehicleSpec::from_stats(stats, BOX).transmission;
    assert_eq!(split(&stats).torque_split, 0.0);
    stats.drive = Drive::Front;
    assert_eq!(split(&stats).torque_split, 1.0);
    stats.drive = Drive::All;
    stats.front_torque_share = 0.3;
    let all = split(&stats);
    assert_eq!((all.torque_split, all.differential[2]), (0.3, 0.5));
}

#[test]
fn the_brakes_convert_back_to_the_stats_newton_metres() {
    let spec = VehicleSpec::from_stats(&sports_car(), BOX);
    assert!((spec.brakes.brake_torque(0) - 1300.0).abs() < 1.0, "{}", spec.brakes.brake_torque(0));
    assert!((spec.brakes.brake_torque(1) - 1350.0).abs() < 1.0);
    assert!((spec.brakes.ebrake_torque() - 1500.0).abs() < 1.0);
}

#[test]
fn grip_is_scaled_from_the_reference_tyre() {
    let spec = VehicleSpec::from_stats(&CarStats { tire_grip: [1.65, 0.825], ..CarStats::default() }, BOX);
    assert!((spec.tires.static_grip[0] - 2.075).abs() < 1e-4);
    assert!((spec.tires.dynamic_grip[0] - 1.85).abs() < 1e-4);
    assert!((spec.tires.static_grip[1] - 1.0375).abs() < 1e-4);
    assert!((spec.tires.grip_scale[0] - 1.0).abs() < 1e-5);
}

#[test]
fn nitrous_and_induction_are_present_only_when_the_stats_give_them() {
    let none = VehicleSpec::from_stats(&sports_car(), BOX);
    assert!(!none.nos.present());
    assert_eq!(none.induction.kind(), crate::induction::InductionKind::None);
    let stats =
        CarStats { nitrous: true, nitrous_boost: 0.9, nitrous_seconds: 6.0, induction_boost: 0.3, ..sports_car() };
    let both = VehicleSpec::from_stats(&stats, BOX);
    assert!(both.nos.present());
    assert_eq!((both.nos.torque_boost, both.nos.nos_capacity), (0.9, 6.0));
    assert_eq!(both.induction.kind(), crate::induction::InductionKind::Turbo);
}

#[test]
fn the_solved_drag_is_a_real_drag_and_balances_the_top_speed() {
    let spec = VehicleSpec::from_stats(&sports_car(), BOX);
    let drag = spec.aero.drag_coefficient;
    // 0.5 rho Cd A is about 0.3 to 0.6 for a car.
    assert!((0.2..0.9).contains(&drag), "{drag}");
    // At the target speed in top gear the wheel force is the drag force: 440 N m near 5800 rpm x 2.44 / 0.343.
    let wheel_force = 435.0 * 0.71 * 3.44 * 0.9 / 0.343;
    assert!((drag * 84.5 * 84.5 / wheel_force - 1.0).abs() < 0.12, "{drag} {wheel_force}");
}

#[test]
fn a_top_speed_beyond_the_rev_limiter_still_gives_a_drag() {
    let stats = CarStats { top_speed: Some(400.0), ..CarStats::default() };
    let spec = VehicleSpec::from_stats(&stats, BOX);
    assert!(spec.aero.drag_coefficient > 0.0 && spec.aero.drag_coefficient.is_finite());
}

#[test]
fn the_car_built_from_stats_runs_near_its_top_speed() {
    let spec = VehicleSpec::from_stats(&sports_car(), BOX);
    let top = measure(&spec).top_speed;
    assert!((0.6 * 84.5..1.1 * 84.5).contains(&top), "{top} m/s against 84.5");
}

#[test]
fn a_stronger_engine_makes_a_faster_and_quicker_car() {
    let stats = CarStats { top_speed: None, ..sports_car() };
    let stock = measure(&VehicleSpec::from_stats(&stats, BOX));
    let tuned = measure(&VehicleSpec::from_stats(&upgrade(&stats, [0, 0, 0, 0, MAX_LEVEL, 0, 0]), BOX));
    assert!(tuned.top_speed > stock.top_speed, "{} {}", tuned.top_speed, stock.top_speed);
}

#[test]
fn an_upgrade_keeps_the_drag_a_top_speed_gave_the_stock_car() {
    let stats = sports_car();
    let stock_drag = VehicleSpec::from_stats(&stats, BOX).aero.drag_coefficient;
    let tuned = upgrade(&stats, [0, 0, 0, 0, 2, 0, 0]);
    assert_eq!(tuned.top_speed, None);
    assert_eq!(VehicleSpec::from_stats(&tuned, BOX).aero.drag_coefficient, stock_drag);
    // No upgrade changes nothing, the target included.
    assert_eq!(upgrade(&stats, [0; 7]), stats);
}

#[test]
fn each_category_changes_what_it_names_by_a_share_of_the_cars_own_value() {
    let stats = sports_car();
    let one = |category: Category, level: u8| {
        let mut levels = [0; 7];
        levels[category as usize] = level;
        upgrade(&stats, levels)
    };
    assert!((one(Category::Tires, 2).tire_grip[0] - 1.5 * 1.10).abs() < 1e-5);
    assert!((one(Category::Brakes, 1).brake_torque_nm[1] - 1350.0 * 1.10).abs() < 1e-2);
    assert!((one(Category::Engine, 3).torque_curve[2][1] - 440.0 * 1.18).abs() < 1e-2);
    assert!((one(Category::Transmission, 3).gear_efficiency - 0.99).abs() < 1e-5);
    assert!((one(Category::Induction, 2).induction_boost - 0.16).abs() < 1e-6);
    let suspension = one(Category::Suspension, 3);
    assert!(suspension.cg_height_m < stats.cg_height_m && suspension.ride_frequency_hz[0] > stats.ride_frequency_hz[0]);
    // Levels beyond the maximum count as the maximum.
    assert_eq!(one(Category::Engine, 9).torque_curve, one(Category::Engine, MAX_LEVEL).torque_curve);
}

#[test]
fn the_nitrous_kit_builds_on_the_factory_nitrous_or_on_the_generic_kit() {
    let mut levels = [0; 7];
    levels[Category::Nitrous as usize] = 1;
    let kit = upgrade(&CarStats::default(), levels);
    assert!(kit.nitrous && (kit.nitrous_boost, kit.nitrous_seconds) == (0.4, 4.0));
    let factory = CarStats { nitrous: true, nitrous_boost: 0.9, nitrous_seconds: 6.0, ..CarStats::default() };
    assert_eq!(upgrade(&factory, levels).nitrous_boost, 0.9);
    levels[Category::Nitrous as usize] = 3;
    let top = upgrade(&factory, levels);
    assert!((top.nitrous_boost - 0.9 * 1.3).abs() < 1e-5 && (top.nitrous_seconds - 6.0 * 1.5).abs() < 1e-5);
}

#[test]
fn the_handling_score_follows_grip_and_the_height_of_the_centre_of_gravity() {
    let grippy = CarStats { tire_grip: [1.8, 1.8], ..sports_car() };
    let slippery = CarStats { tire_grip: [1.0, 1.0], ..sports_car() };
    assert!(handling_index(&grippy) > handling_index(&sports_car()));
    assert!(handling_index(&sports_car()) > handling_index(&slippery));
    let tall = CarStats { cg_height_m: 0.9, ..sports_car() };
    assert!(handling_index(&tall) < handling_index(&sports_car()));
    for stats in [grippy, slippery, tall, CarStats::default()] {
        assert!((0.0..=1.0).contains(&handling_index(&stats)));
    }
}

#[test]
fn a_tyre_upgrade_raises_the_handling_score() {
    let stats = sports_car();
    let mut levels = [0; 7];
    levels[Category::Tires as usize] = MAX_LEVEL;
    assert!(handling_index(&upgrade(&stats, levels)) > handling_index(&stats));
}

#[test]
fn the_curve_is_linear_between_points_and_flat_outside() {
    let stats = CarStats { torque_curve: vec![[1000.0, 100.0], [2000.0, 200.0]], ..CarStats::default() };
    assert_eq!(stats.torque_at(500.0), 100.0);
    assert!((stats.torque_at(1500.0) - 150.0).abs() < 1e-4);
    assert_eq!(stats.torque_at(9000.0), 200.0);
    assert_eq!(stats.peak_torque_nm(), 200.0);
}
