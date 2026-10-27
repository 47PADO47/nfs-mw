use blackbox_modding::content::SparkLayer;

use super::super::fixtures::layer;
use super::*;

fn birth(strength: f32, hit: bool) -> Birth {
    Birth { point: Vec3::new(1.0, 2.0, 3.0), normal: Vec3::Z, owner: Vec3::ZERO, strength, hit }
}

#[test]
fn uniform_samples_are_in_range_and_differ_by_seed_and_slot() {
    for seed in 0..50 {
        for slot in 0..8 {
            let value = uniform(seed, slot);
            assert!((0.0..1.0).contains(&value));
        }
    }
    assert_ne!(uniform(1, 0), uniform(2, 0));
    assert_ne!(uniform(1, 0), uniform(1, 1));
}

#[test]
fn an_impact_flies_as_fast_as_it_is_strong_along_the_normal() {
    let layer = layer();
    for (strength, speed) in [(1.0, 8.0), (0.5, 4.0), (0.25, 2.0)] {
        let spark = Spark::spawn(&layer, 0, &birth(strength, true), 7);
        assert!(spark.velocity.abs_diff_eq(Vec3::Z * speed, 1e-4), "{:?}", spark.velocity);
        assert!(spark.point.abs_diff_eq(Vec3::new(1.0, 2.0, 3.02), 1e-5), "born a little off the surface");
    }
}

#[test]
fn a_scrape_changes_the_rate_and_not_the_speed() {
    let spark = Spark::spawn(&layer(), 0, &birth(0.25, false), 7);
    assert!(spark.velocity.abs_diff_eq(Vec3::Z * 8.0, 1e-4), "{:?}", spark.velocity);
}

#[test]
fn the_spread_is_a_cone_around_the_normal_and_the_variances_shorten() {
    let layer = SparkLayer { spread: 30.0, speed_variance: 0.5, life_variance: 0.5, ..layer() };
    let (mut widest, mut slowest, mut shortest) = (0.0f32, f32::MAX, f32::MAX);
    for seed in 0..400 {
        let spark = Spark::spawn(&layer, 0, &birth(1.0, true), seed);
        let angle = spark.velocity.normalize().dot(Vec3::Z).clamp(-1.0, 1.0).acos().to_degrees();
        widest = widest.max(angle);
        slowest = slowest.min(spark.velocity.length());
        shortest = shortest.min(spark.life);
        assert!(angle <= 30.001 && spark.velocity.length() <= 8.0 + 1e-4 && spark.life <= 1.0);
    }
    assert!(widest > 25.0, "the cone is used: {widest}");
    assert!(slowest < 5.0 && shortest < 0.7, "{slowest} {shortest}");
}

#[test]
fn the_cars_velocity_is_taken_by_the_layers_share() {
    let layer = SparkLayer { inherit: 0.5, speed: 0.0, ..layer() };
    let moving = Birth { owner: Vec3::new(20.0, 0.0, 0.0), ..birth(1.0, true) };
    let spark = Spark::spawn(&layer, 0, &moving, 3);
    assert!(spark.velocity.abs_diff_eq(Vec3::new(10.0, 0.0, 0.0), 1e-4));
}

#[test]
fn gravity_pulls_down_and_drag_slows() {
    let falling = SparkLayer { gravity: 10.0, speed: 0.0, ..layer() };
    let mut spark = Spark::spawn(&falling, 0, &birth(1.0, true), 1);
    spark.step(&falling, 0.1);
    assert!((spark.velocity.z + 1.0).abs() < 1e-5, "{:?}", spark.velocity);
    assert!(spark.point.z < 3.02);
    let dragged = SparkLayer { drag: 0.5, ..layer() };
    let mut spark = Spark::spawn(&dragged, 0, &birth(1.0, true), 1);
    spark.step(&dragged, 0.1);
    // 8 m/s loses 0.1 * 0.5 * 8 = 40 percent.
    assert!((spark.velocity.z - 4.8).abs() < 1e-4, "{:?}", spark.velocity);
}

#[test]
fn a_spark_lives_for_its_life_and_a_nan_one_dies() {
    let layer = layer();
    let mut spark = Spark::spawn(&layer, 0, &birth(1.0, true), 1);
    assert!(spark.alive());
    spark.step(&layer, 0.5);
    assert!(spark.alive());
    spark.step(&layer, 0.5);
    assert!(!spark.alive(), "age reached life");
    spark.age = 0.0;
    spark.point = Vec3::NAN;
    assert!(!spark.alive());
}

#[test]
fn the_keys_give_the_segment_and_straight_lines() {
    let keys = [0.0, 0.5, 1.0];
    assert_eq!(segment(&keys, 0.0), (0, 0.0));
    assert_eq!(segment(&keys, 0.25), (0, 0.5));
    assert_eq!(segment(&keys, 0.5), (1, 0.0));
    assert_eq!(segment(&keys, 0.75), (1, 0.5));
    assert_eq!(segment(&keys, 1.0), (1, 1.0));
    assert_eq!(segment(&keys, 2.0), (1, 1.0));
    assert_eq!(segment(&[0.2, 0.6], 0.1), (0, 0.0), "before the first key is the first value");
    assert_eq!(segment(&[0.5], 0.5), (0, 0.0), "a single key is no track");
    assert_eq!(segment(&[], 0.5), (0, 0.0));
}

#[test]
fn the_look_follows_the_keys() {
    let layer = layer();
    let mut spark = Spark::spawn(&layer, 0, &birth(1.0, true), 1);
    let start = spark.look(&layer);
    assert_eq!((start.size, start.color, start.brightness), (0.1, [255, 100, 0, 255], 10.0));
    spark.age = 0.25;
    let early = spark.look(&layer);
    assert!(
        (early.size - 0.15).abs() < 1e-6 && early.color == [255, 150, 50, 192] && (early.brightness - 7.0).abs() < 1e-5
    );
    spark.age = 0.5;
    let middle = spark.look(&layer);
    assert_eq!((middle.size, middle.color, middle.brightness), (0.2, [255, 200, 100, 128], 4.0));
    spark.age = 0.75;
    let late = spark.look(&layer);
    assert!((late.size - 0.1).abs() < 1e-6 && late.color[3] == 64);
}

#[test]
fn a_sheet_shows_a_random_cell_or_plays_its_frames() {
    let plain = layer();
    let whole = Spark::spawn(&plain, 0, &birth(1.0, true), 1).window(&plain);
    assert_eq!(whole, [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]);
    let sheet = SparkLayer { grid: 4, fps: 0, random_frame: true, ..layer() };
    let mut cells = std::collections::BTreeSet::new();
    for seed in 0..64 {
        let window = Spark::spawn(&sheet, 0, &birth(1.0, true), seed).window(&sheet);
        assert!((window[1][0] - window[0][0] - 0.25).abs() < 1e-6 && (window[2][1] - window[0][1] - 0.25).abs() < 1e-6);
        cells.insert(((window[0][0] * 4.0).round() as u32, (window[0][1] * 4.0).round() as u32));
    }
    assert!(cells.len() > 4, "a random cell for each particle: {}", cells.len());
    let fixed = SparkLayer { random_frame: false, ..sheet.clone() };
    assert_eq!(Spark::spawn(&fixed, 0, &birth(1.0, true), 5).window(&fixed)[0], [0.0, 0.0], "the first cell");
    let playing = SparkLayer { fps: 8, random_frame: false, ..sheet };
    let mut spark = Spark::spawn(&playing, 0, &birth(1.0, true), 5);
    spark.age = 0.25; // frame 2 of 16 at 8 a second
    assert_eq!(spark.window(&playing)[0], [0.5, 0.0]);
    spark.age = 0.5; // frame 4: the second row
    assert_eq!(spark.window(&playing)[0], [0.0, 0.25]);
}
