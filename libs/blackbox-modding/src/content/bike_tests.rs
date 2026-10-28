use blackbox_vehicle::bike::BikeSpec;

use super::*;

fn numbers(values: &[f32]) -> String {
    let list: Vec<String> = values.iter().map(|v| format!("{v:?}")).collect();
    format!("[{}]", list.join(", "))
}

/// The tyre of the generic example bike, as the manifest writes it.
fn tyre(peak: f32) -> String {
    let mut a = [0.0_f32; MF_A];
    a[0] = 1.2;
    a[1] = -10.0;
    a[2] = peak;
    a[3] = 1200.0;
    a[4] = 6.0;
    a[5] = 0.002;
    a[8] = -0.01;
    let mut b = [0.0_f32; MF_B];
    b[0] = 1.5;
    b[1] = -10.0;
    b[2] = 1250.0;
    b[4] = 200.0;
    b[6] = -0.1;
    b[8] = 0.1;
    format!("a = {}\nb = {}\nc = {}\n", numbers(&a), numbers(&b), numbers(&[0.0; MF_C]))
}

fn wheel(table: &str, mass: f32, peak: f32) -> String {
    format!(
        "[bike.{table}]\nradius_m = 0.32\nwidth_m = 0.12\ntoroid_factor = 0.7\nmass_kg = {mass:?}\ninertia_kgm2 = 0.65\n\
         rolling = [0.02, 0.0004, 0.000001]\nlateral_grip_limit = 1.25\ntire_stiffness_n_per_m = 400000.0\n\
         max_tire_deformation_m = 0.03\ncompression_shape_factor = 1.0\nslip_correction = 0.05\n\
         rest_slip_correction = 0.25\nrest_speed_threshold_mps = 6.9444447\n[bike.{table}.tyre]\n{}",
        tyre(peak)
    )
}

const DAMPER: &str = "slow_bump = 1600.0\nfast_bump = 1250.0\nbump_threshold = 0.3\nslow_rebound = 2400.0\n\
                      fast_rebound = 1650.0\nrebound_threshold = 0.3\n";

const LEAN: &str = "[bike.steering.lean]\nk = 1000.0\ndamping_ratio = 3.6\ndamping_multiplier = 1.0\nsteer_k = 1000.0\n\
    steer_damping_ratio = 2.5\ntarget_k = 1000.0\ntarget_damping_ratio = 2.25\ntime_scale = 1.0\n\
    time_scale_attenuation = 0.0\ntime_scale_sensitivity = 0.0\n\
    time_scale_speed = [[2.7777777, 1.0], [55.555557, 0.65]]\ntime_scale_input = [[-1.0, 0.425], [0.0, 1.0], [1.0, 0.7]]\n\
    g_tuning = [[0.0, 1.0], [1.1, 0.9]]\nskid_range_rad = [0.17453292, 0.6981317]\nskid_linearity = 1.8\n\
    skid_reduction = [[0.0, 0.0], [8.333333, 0.2]]\nstopped_limit_rad = 0.017453292\nstopped_speed_mps = 8.333333\n\
    blend_range_mps = [2.0, 4.0]\nrider_mass_scale = 1.0\nhelp_speed = [[0.0, 0.0], [2.7777777, 0.2]]\nhelp_limit = 1.0\n";

const PITCH: &str = "deadzone = 0.3\nmax_angle_rad = 0.61086524\nspeed_ramp_mps = [0.0, 4.1666665]\n\
    lean_help = [0.2, 1.0]\nlookahead_s = 0.0\nmin_speed_mps = 4.1666665\nacceleration = 10.0\n\
    acceleration_time_s = 0.1\nground_range_m = [0.0, 1.0]\nlean_limit_rad = 0.017453292\n\
    lean_pitch_range_rad = [0.0, 0.6981317]\n";

/// A complete `[bike]` table: the generic example bike of `BikeSpec::example`.
fn table() -> String {
    format!(
        "[bike]\ndrag = 0.15\ngyro_roll_scale = 1.0\ngyro_yaw_scale = 1.0\n\
         [bike.chassis]\nmass_kg = 113.0\nrider_mass_kg = 70.0\nrider_offset_m = [0.0, 0.4, -0.05]\n\
         rider_forward_shift_m = [0.0, 0.0, 0.1]\nrider_backward_shift_m = [0.0, 0.0, -0.1]\nrider_weight_rate = 6.0\nrider_k_n_per_m = 20000.0\nrider_damp = 5000.0\n\
         half_dimensions_m = [0.42, 0.64, 1.06]\ntensor_scale = [0.4, 0.45, 0.2]\n\
         cg_height_m = 0.62\nfront_weight = 0.5\nwheelbase_m = 1.47\nrake_rad = 0.4537856\ntrail_m = 0.105\n\
         {front}{rear}\
         [bike.front_suspension]\nmax_travel_m = 0.28\nspring_n_per_m = 8800.0\ntop_out_stiffness_n_per_m = 3800.0\n\
         top_out_length_m = 0.06\nrest_offset_m = 0.012\nmax_speed_mps = 5.0\n\
         [bike.front_suspension.damper]\n{DAMPER}\
         [bike.front_suspension.hydraulic]\nstiffness_n_per_m = 400.0\nfactor = 1.0\nlinearity = 2.0\n\
         [bike.front_suspension.coupling]\nbrake = 35.0\nbrake_range = [0.0, 1.0]\n\
         [bike.rear_suspension]\nmax_travel_m = 0.26\nspring_n_per_m = 48000.0\ntop_out_stiffness_n_per_m = 17000.0\n\
         top_out_length_m = 0.035\nrest_offset_m = 0.01\nmax_speed_mps = 5.0\nleverage = [[0.0, 2.8], [1.0, 1.97]]\n\
         [bike.rear_suspension.damper]\nslow_bump = 7500.0\nfast_bump = 1250.0\nbump_threshold = 0.2\n\
         slow_rebound = 10500.0\nfast_rebound = 1650.0\nrebound_threshold = 0.2\n\
         [bike.rear_suspension.hydraulic]\nstiffness_n_per_m = 200.0\nfactor = 1.0\nlinearity = 2.8\n\
         [bike.rear_suspension.coupling]\nchain = -0.014\nchain_range = [1.0, 0.6]\ntraction = -0.15\n\
         traction_range = [1.0, -0.4]\nbrake = 30.0\nbrake_range = [0.0, 1.0]\n\
         [bike.engine]\npeak_torque_nm = 46.0\n\
         torque_curve = [[2300.0, 0.7], [5000.0, 0.85], [7500.0, 1.0], [9500.0, 0.9], [11000.0, 0.7]]\n\
         friction_torque_nm = 12.0\nfriction_curve = [[0.0, 0.0], [1.0, 1.0], [1.5, 2.0]]\nidle_rpm = 2300.0\n\
         max_rpm = 11000.0\nmoi_kgm2 = 0.015\nthrottle_at_idle = 0.15\nlimiter_cut_time_s = 0.02\n\
         [bike.transmission]\ngears = [2.1, 1.65, 1.35, 1.12, 0.96]\nfinal_ratio = 7.2\nefficiency = 1.0\n\
         up_tolerance = [1.0, 1.0, 1.0, 1.0, 1.0]\ndown_tolerance = [0.4, 0.9, 1.0, 1.0, 1.0]\n\
         prevent_up_lean = [0.5, 30.0]\nprevent_down_lean = [0.5, 50.0]\nprevent_down_throttle = [1.0, 0.5]\nup_shift_time_s = 0.06\ndown_shift_time_s = 0.15\nblip_throttle = 0.3\n\
         clutch_linearity = 1.0\nclutch_max_factor = 1.0\nslipper_range_nm = [25.0, 40.0]\nslipper_intensity = 1.0\n\
         release_speed_mps = 13.888889\nfeather_factor = 0.85\nfeather_max_factor = 0.7\nlaunch_rpm = [0.4, 0.98]\n\
         [bike.transmission.feather]\ndelta_time_s = 0.05\n\
         [bike.brakes]\nfront_force_n = 1800.0\nrear_force_n = 1000.0\njoined_factor = 1.0\n\
         [bike.brakes.abs]\nfront_intensity = 1.0\nrear_intensity = 1.0\nspeed_range_mps = [2.7777777, 6.9444447]\n\
         front_slip = [-0.3, -0.5]\nrear_slip = [-0.5, -1.0]\n\
         [bike.steering]\nmax_angle_rad = 0.7\nresponse = 1000.0\ndamping_ratio = 3.0\nmax_lean_rad = 1.2217305\n\
         {LEAN}\
         [bike.fall]\nlean_rad = 1.2217305\npitch_rad = 1.134464\nspeed_direction_rad = 1.4835299\n\
         angular_speed = 4.4\nmin_speed_mps = 2.7777777\nblock_ratio = 0.975\nblock_min_speed_mps = 11.111111\n\
         block_time_s = 1.0\nblock_wait_s = 5.0\n\
         [bike.tcs]\nslip = [0.09, 0.2]\nspeed_range_mps = [0.0, 1.3888889]\n\
         [bike.wheelie]\nthrottle_pitch_rad = [0.0, 0.0698132]\nforced_throttle_pitch_rad = [0.5759587, 0.7853982]\n\
         throttle_pitch_speed = [0.2, 0.6]\nbrake_pitch_rad = [0.0, 0.0698132]\nbrake_pitch_speed = [0.0, 0.6]\n\
         [bike.wheelie.wheelie]\n{PITCH}\
         [bike.wheelie.stoppie]\n{PITCH}",
        front = wheel("front", 17.0, 1330.0),
        rear = wheel("rear", 19.0, 1360.0),
    )
}

fn bike_car() -> String {
    format!("kind = \"bike\"\nname = \"Test bike\"\n{}", table())
}

fn bike(text: &str) -> Result<CarManifest, ContentError> {
    parse_car("testbike", text)
}

fn car(text: &str) -> Result<CarManifest, ContentError> {
    parse_car("fastcoupe", text)
}

#[test]
fn a_complete_bike_is_read() {
    let manifest = bike(&bike_car()).unwrap();
    assert_eq!(manifest.kind, CarKind::Bike);
    assert_eq!(manifest.role, Role::Player);
    let table = manifest.bike.expect("the bike table");
    assert_eq!(table.chassis.mass_kg, 113.0);
    assert_eq!(table.chassis.rider_mass_kg, 70.0);
    assert_eq!(table.rear_suspension.leverage, vec![[0.0, 2.8], [1.0, 1.97]]);
    assert!(table.front_suspension.leverage.is_empty());
    assert_eq!(table.front.tyre.a[2], 1330.0);
    assert!(table.wheelie.is_some());
}

#[test]
fn a_plain_car_is_a_car() {
    let manifest = car("").unwrap();
    assert_eq!(manifest.kind, CarKind::Car);
    assert!(manifest.bike.is_none());
}

#[test]
fn the_table_converts_to_the_spec_the_dynamics_take() {
    let text = bike_car();
    let table = bike(&text).unwrap().bike.unwrap();
    let spec = BikeSpec::from(&table);
    let example = BikeSpec::example();
    assert_eq!(spec.chassis.mass, example.chassis.mass);
    assert_eq!(spec.chassis.rider_mass, example.chassis.rider_mass);
    assert_eq!(spec.chassis.rider_offset, example.chassis.rider_offset);
    assert_eq!(spec.chassis.half_dimensions, example.chassis.half_dimensions);
    assert_eq!(spec.chassis.tensor_scale, example.chassis.tensor_scale);
    assert!((spec.chassis.rake - example.chassis.rake).abs() < 1e-6);
    assert_eq!(spec.front.tyre, example.front.tyre);
    assert_eq!(spec.rear.tyre, example.rear.tyre);
    assert_eq!(spec.front.mass, 17.0);
    assert_eq!(spec.rear.mass, 19.0);
    assert_eq!(spec.front.tire_stiffness, example.front.tire_stiffness);
    assert_eq!(spec.rear_suspension.leverage, example.rear_suspension.leverage);
    assert_eq!(spec.rear_suspension.hydraulic, example.rear_suspension.hydraulic);
    assert_eq!(spec.rear_suspension.coupling, example.rear_suspension.coupling);
    assert_eq!(spec.front_suspension.damper, example.front_suspension.damper);
    assert_eq!(spec.front_suspension.top_out_length, example.front_suspension.top_out_length);
    assert_eq!(spec.engine.torque_curve, example.engine.torque_curve);
    assert_eq!(spec.engine.friction_curve, example.engine.friction_curve);
    assert_eq!(spec.engine.moi, example.engine.moi);
    assert_eq!(spec.transmission.gears, example.transmission.gears);
    assert_eq!(spec.transmission.clutch_max_factor, example.transmission.clutch_max_factor);
    assert_eq!(spec.brakes.front_force, example.brakes.front_force);
    assert_eq!(spec.brakes.rear_force, example.brakes.rear_force);
    assert_eq!(spec.brakes.joined_factor, example.brakes.joined_factor);
    assert_eq!(spec.steering.lean.k, example.steering.lean.k);
    assert_eq!(spec.steering.lean.g_tuning, example.steering.lean.g_tuning);
    assert_eq!(spec.steering.lean.help_speed.len(), example.steering.lean.help_speed.len());
    assert!((spec.steering.max_lean - example.steering.max_lean).abs() < 1e-5);
    assert!((spec.fall.lean - example.fall.lean).abs() < 1e-5);
    let (assist, expected) = (spec.wheelie.as_ref().unwrap(), example.wheelie.as_ref().unwrap());
    assert_eq!(assist.wheelie.deadzone, expected.wheelie.deadzone);
    assert_eq!(assist.wheelie.acceleration, expected.wheelie.acceleration);
    assert_eq!(spec.transmission.up_tolerance, example.transmission.up_tolerance);
    assert_eq!(spec.transmission.down_tolerance, example.transmission.down_tolerance);
    assert_eq!(spec.transmission.prevent_down_throttle, example.transmission.prevent_down_throttle);
    assert_eq!(spec.transmission.feather.delta_time, example.transmission.feather.delta_time);
    assert_eq!(spec.transmission.feather.exit_tolerance, example.transmission.feather.exit_tolerance);
    assert_eq!(spec.fall.block_ratio, example.fall.block_ratio);
    assert_eq!((spec.chassis.rider_k, spec.chassis.rider_damp), (20_000.0, 5_000.0));
    assert_eq!(spec.tcs.expect("the traction control").slip, [0.09, 0.2]);
    assert_eq!(spec.front.rolling_slip_linearity, 1.0, "the exponent defaults to 1");
    assert_eq!(spec.gyro_roll_scale, example.gyro_roll_scale);
    assert_eq!(spec.drag, example.drag);
    assert_eq!(spec.total_mass(), 113.0 + 70.0);
}

#[test]
fn a_bike_without_a_rider_or_a_wheelie_assist_is_a_bike_without_them() {
    let start = table().find("[bike.wheelie]").unwrap();
    let text = bike_car().replace(&table()[start..], "");
    let text = text.replace("rider_mass_kg = 70.0\n", "");
    let spec = BikeSpec::from(&bike(&text).unwrap().bike.unwrap());
    assert_eq!(spec.chassis.rider_mass, 0.0, "no rider unless the manifest says so");
    assert!(spec.wheelie.is_none());
    assert_eq!(spec.total_mass(), 113.0);
}

#[test]
fn the_keys_whose_zero_means_none_may_be_left_out() {
    let mut text = bike_car();
    for key in ["tire_stiffness_n_per_m = 400000.0\n", "gyro_roll_scale = 1.0\n", "release_speed_mps = 13.888889\n"] {
        text = text.replace(key, "");
    }
    let spec = BikeSpec::from(&bike(&text).unwrap().bike.unwrap());
    assert_eq!(spec.front.tire_stiffness, 0.0);
    assert_eq!(spec.gyro_roll_scale, 0.0);
    assert_eq!(spec.transmission.release_speed, 0.0);
}

#[test]
fn a_key_that_is_left_out_refuses_the_bike_and_is_named() {
    let keys = [
        "drag = 0.15\n",
        "wheelbase_m = 1.47\n",
        "final_ratio = 7.2\n",
        "friction_torque_nm = 12.0\n",
        "moi_kgm2 = 0.015\n",
    ];
    for key in keys {
        let error = bike(&bike_car().replace(key, "")).unwrap_err();
        let ContentError::Parse(message) = error else { panic!("expected a parse error for {key:?}") };
        let name = key.split(' ').next().unwrap();
        assert!(message.contains(name), "{message}");
    }
}

#[test]
fn an_unknown_key_refuses_the_bike() {
    let text = bike_car().replace("trail_m = 0.105\n", "trail_m = 0.105\nswingarm_m = 0.6\n");
    let ContentError::Parse(message) = bike(&text).unwrap_err() else { panic!("expected a parse error") };
    assert!(message.contains("swingarm_m"), "{message}");
}

#[test]
fn the_old_brake_keys_are_refused() {
    let text = bike_car().replace("front_force_n = 1800.0\n", "force_n = 1800.0\nfront_force_n = 1800.0\n");
    assert!(matches!(bike(&text), Err(ContentError::Parse(_))));
}

#[test]
fn a_tyre_set_of_the_wrong_length_is_refused() {
    let text = bike_car().replacen("c = [0.0, 0.0", "c = [0.0, 0.0, 0.0, 0.0", 1);
    assert!(matches!(bike(&text), Err(ContentError::Parse(_))));
}

/// The first keys of the rear wheel, which no other table has (its mass is 19 kg).
const REAR_WHEEL_HEAD: &str = "radius_m = 0.32\nwidth_m = 0.12\ntoroid_factor = 0.7\nmass_kg = 19.0";

#[test]
fn a_number_out_of_range_names_its_key() {
    let wide = REAR_WHEEL_HEAD.replace("radius_m = 0.32", "radius_m = 3.0");
    let cases = [
        ("mass_kg = 113.0", "mass_kg = 9000.0", "bike.chassis.mass_kg"),
        ("rake_rad = 0.4537856", "rake_rad = 26.0", "bike.chassis.rake_rad"),
        ("front_weight = 0.5", "front_weight = 1.5", "bike.chassis.front_weight"),
        (REAR_WHEEL_HEAD, wide.as_str(), "bike.rear.radius_m"),
        ("max_travel_m = 0.28", "max_travel_m = 2.8", "bike.front_suspension.max_travel_m"),
        ("front_force_n = 1800.0", "front_force_n = 90000.0", "bike.brakes.front_force_n"),
        ("max_rpm = 11000.0", "max_rpm = 1000.0", "bike.engine.max_rpm"),
        ("moi_kgm2 = 0.015", "moi_kgm2 = 900.0", "bike.engine.moi_kgm2"),
        ("efficiency = 1.0", "efficiency = 0.1", "bike.transmission.efficiency"),
        ("clutch_max_factor = 1.0", "clutch_max_factor = 90.0", "bike.transmission.clutch_max_factor"),
        ("max_lean_rad = 1.2217305", "max_lean_rad = 1.45", "bike.steering.max_lean_rad"),
        ("angular_speed = 4.4", "angular_speed = 400.0", "bike.fall.angular_speed"),
    ];
    for (from, to, expected) in cases {
        let text = bike_car().replacen(from, to, 1);
        assert_ne!(text, bike_car(), "{from:?} is in the table");
        let error = bike(&text).unwrap_err();
        assert!(matches!(&error, ContentError::Field { field, .. } if *field == expected), "{expected}: {error}");
    }
}

#[test]
fn the_curves_have_to_rise_or_fall() {
    let cases = [
        ("[2300.0, 0.7], [5000.0, 0.85]", "[5000.0, 0.7], [2300.0, 0.85]", "bike.engine.torque_curve"),
        ("gears = [2.1, 1.65", "gears = [1.5, 1.65", "bike.transmission.gears"),
        ("down_tolerance = [0.4", "down_tolerance = [40.0", "bike.transmission.down_tolerance"),
        ("[[0.0, 2.8], [1.0, 1.97]]", "[[1.0, 2.8], [0.0, 1.97]]", "bike.rear_suspension.leverage"),
        ("g_tuning = [[0.0, 1.0], [1.1, 0.9]]", "g_tuning = [[1.1, 1.0], [0.0, 0.9]]", "bike.steering.lean"),
    ];
    for (from, to, expected) in cases {
        let text = bike_car().replacen(from, to, 1);
        assert_ne!(text, bike_car(), "{from:?} is in the table");
        let error = bike(&text).unwrap_err();
        assert!(matches!(&error, ContentError::Field { field, .. } if *field == expected), "{expected}: {error}");
    }
}

#[test]
fn a_non_finite_number_is_refused() {
    let text = bike_car().replace("trail_m = 0.105", "trail_m = nan");
    let error = bike(&text).unwrap_err();
    assert!(matches!(&error, ContentError::Field { field: "bike.chassis.trail_m", .. }), "{error}");
}

#[test]
fn the_bike_table_and_the_kind_go_together() {
    assert_eq!(bike(&table()), Err(ContentError::BikeWithoutKind));
    assert_eq!(bike("kind = \"bike\"\n"), Err(ContentError::BikeWithoutTable));
    assert!(matches!(bike("kind = \"truck\"\n"), Err(ContentError::Parse(_))));
}

#[test]
fn a_bike_is_a_player_car_only() {
    let cop = format!("role = \"cop\"\n{}", bike_car());
    assert_eq!(bike(&cop), Err(ContentError::BikeNotPlayer));
    let both = format!("role = \"both\"\n{}", bike_car());
    assert_eq!(bike(&both), Err(ContentError::BikeNotPlayer));
    let traffic = format!("role = \"traffic\"\n{}[traffic]\nclass = \"compact\"\n", bike_car());
    assert_eq!(bike(&traffic), Err(ContentError::BikeNotPlayer));
    let on_roads = format!("{}[traffic]\nclass = \"compact\"\n", bike_car());
    assert_eq!(bike(&on_roads), Err(ContentError::BikeNotPlayer));
}

#[test]
fn a_bike_has_no_rims() {
    // A `[customization]` table needs a body paint of its own to be valid; the rims are what is refused.
    let table = "[customization]\npaints = [{ name = \"Red\", color = [1, 2, 3] }]\n";
    let text = format!("{}{table}rims = [{{ name = \"Five\", model = \"BIKE_RIM1\" }}]\n", bike_car());
    assert_eq!(bike(&text), Err(ContentError::BikeRims));
}

#[test]
fn a_bike_places_its_two_wheels_without_a_face() {
    let wheels = "[wheels.front]\nx_m = 0.7355\nz_m = 0.324\nradius_m = 0.324\nwidth_m = 0.12\n\
                  [wheels.rear]\nx_m = -0.739\nz_m = 0.324\nradius_m = 0.324\nwidth_m = 0.15\n";
    let manifest = bike(&format!("{}{wheels}", bike_car())).unwrap();
    let layout = manifest.wheels.expect("the layout");
    assert_eq!((layout.front.x_m, layout.rear.x_m, layout.front.face_m), (0.7355, -0.739, 0.0));
    // The same table on a car is missing the face.
    assert!(matches!(car(wheels), Err(ContentError::Field { field: "wheels.front.face_m", .. })));
}

#[test]
fn a_stats_table_on_a_bike_is_not_read() {
    let text = format!("{}[stats]\nmass_kg = 200.0\n", bike_car());
    let manifest = bike(&text).unwrap();
    assert_eq!(manifest.stats, Stats::default());
}

const THERMAL: &str = "[bike.front.thermal]\nsections_dependency = 1.0\nsections_angle_rad = 1.0\ninitial_thickness = 1.1\n\
    min_temperature_c = 0.0\nmax_temperature_c = 200.0\nasphalt_importance = 1.0\nwear_length_m = 5000.0\nuniform = true\n\
    static_time_scale = 1.0\ndynamic_time_scale = 1.0\ndecay_distance_factor = 1.0\nwear_intensity = 1.0\n\
    [[bike.front.thermal.sections]]\nefficiency_limit = 0.9\nstatic_coef = 0.009\ndynamic_coef = 0.02\n\
    lat_slip_coef = 0.00001\nlong_slip_coef = 0.00001\nrolling_coef = 0.0002\n\
    temperature_to_efficiency = [[60.3, 0.975], [90.1, 1.0]]\nwear_to_efficiency = [[0.1, 0.985], [1.0, 0.94]]\n\
    temperature_to_wear = [[88.0, 0.95], [150.0, 0.0]]\n";

#[test]
fn a_tyre_with_a_thermal_table_converts_to_the_thermal_spec() {
    let text = bike_car().replacen("[bike.rear]", &format!("{THERMAL}[bike.rear]"), 1);
    let spec = BikeSpec::from(&bike(&text).unwrap().bike.unwrap());
    let thermal = spec.front.thermal.expect("the thermal table");
    assert_eq!(thermal.sections.len(), 1);
    assert_eq!(thermal.sections[0].temperature_to_efficiency, vec![(60.3, 0.975), (90.1, 1.0)]);
    assert!(thermal.uniform && spec.rear.thermal.is_none());
    let bad = text.replace("max_temperature_c = 200.0", "max_temperature_c = -5.0");
    assert!(matches!(bike(&bad), Err(ContentError::Field { field: "bike.front.thermal", .. })));
}

#[test]
fn the_rider_help_table_and_the_target_filter_are_read_and_default_to_the_reference_constructor() {
    let plain = bike(&bike_car()).unwrap().bike.unwrap().wheelie.unwrap().to_spec();
    assert!(plain.help.auto_rider.is_none() && plain.help.launch_speed == 0.0);
    assert_eq!((plain.help.throttle_multiplier, plain.help.intensity), (1.0, 1.0));
    assert_eq!((plain.wheelie.filter_timescale, plain.wheelie.terrain_influence), (0.0, 0.0));
    let text = bike_car().replace(
        "[bike.wheelie.wheelie]",
        "[bike.wheelie.help]\nlaunch_speed_mps = 5.5\nlaunch_throttle_multiplier = 0.0\nthrottle_pitch_electronic = true\n\
         weight_deadzone = 0.75\n[bike.wheelie.help.auto_rider]\nwheelie_range_rad = [0.06, 0.07]\n\
         throttle_curve = [[0.0, 0.0], [0.1, 1.0]]\nengage_speed = 5.0\n[bike.wheelie.wheelie]\n\
         terrain_influence = 1.0\ntarget_filter_timescale = 2.5\n",
    );
    let spec = bike(&text).unwrap().bike.unwrap().wheelie.unwrap().to_spec();
    assert_eq!((spec.help.launch_speed, spec.help.launch_throttle_multiplier), (5.5, 0.0));
    assert!(spec.help.throttle_pitch_electronic && !spec.help.brake_pitch_electronic);
    assert_eq!(spec.help.weight_deadzone, 0.75);
    let auto = spec.help.auto_rider.expect("the auto-rider");
    assert_eq!((auto.wheelie_range, auto.engage_speed, auto.throttle_curve.len()), ([0.06, 0.07], 5.0, 2));
    assert_eq!((spec.wheelie.terrain_influence, spec.wheelie.filter_timescale), (1.0, 2.5));
}
