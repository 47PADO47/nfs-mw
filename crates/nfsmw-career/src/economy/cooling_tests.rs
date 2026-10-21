use super::*;

/// The install's values (`fecooling`, read in this pass).
fn table() -> CoolingTable {
    CoolingTable::new([0.69, 0.98, 0.9, 0.97, 0.7, 0.95, 0.97, 0.97, 0.9, 0.75, 0.99])
}

#[test]
fn a_fitted_part_multiplies_the_heat_by_its_factor() {
    let mut record = CareerRecord::new(0);
    record.heat = 4.0;
    record.cool_for_part(&table(), CoolingPart::Paint, SHOP_EXTRA);
    assert!((record.heat - 2.8).abs() < 1e-5);
    record.cool_for_part(&table(), CoolingPart::BodyKit, SHOP_EXTRA);
    assert!((record.heat - 2.8 * 0.69).abs() < 1e-5);
}

#[test]
fn the_backroom_cools_a_quarter_more() {
    let mut record = CareerRecord::new(0);
    record.heat = 4.0;
    record.cool_for_part(&table(), CoolingPart::Vinyl, BACKROOM_EXTRA);
    assert!((record.heat - 4.0 * 0.75 * 0.75).abs() < 1e-5);
}

#[test]
fn a_car_with_no_heat_stays_cool_and_a_bad_heat_is_left_alone() {
    let mut record = CareerRecord::new(0);
    record.cool_for_part(&table(), CoolingPart::Paint, SHOP_EXTRA);
    assert_eq!(record.heat, 0.0);
    record.heat = f32::NAN;
    record.cool_for_part(&table(), CoolingPart::Paint, SHOP_EXTRA);
    assert!(record.heat.is_nan());
    record.heat = -1.0;
    record.cool_for_part(&table(), CoolingPart::Paint, SHOP_EXTRA);
    assert_eq!(record.heat, -1.0);
}

#[test]
fn the_preview_applies_every_part_in_turn_and_changes_nothing() {
    let parts = [CoolingPart::Paint, CoolingPart::Spoiler, CoolingPart::Decal];
    let heat = preview_heat(5.0, &table(), &parts, SHOP_EXTRA);
    assert!((heat - 5.0 * 0.7 * 0.9 * 0.98).abs() < 1e-4);
    assert_eq!(preview_heat(5.0, &table(), &[], SHOP_EXTRA), 5.0);
}

#[test]
fn every_part_kind_reads_its_own_factor() {
    let t = table();
    let seen: Vec<f32> = CoolingPart::ALL.iter().map(|part| t.factor(*part)).collect();
    assert_eq!(seen, vec![0.69, 0.98, 0.9, 0.97, 0.7, 0.95, 0.97, 0.97, 0.9, 0.75, 0.99]);
}

#[test]
fn the_meter_of_a_cold_car_shows_x1_and_nothing_lit() {
    let meter = HeatMeter::of(0.0);
    assert_eq!(meter.multiplier, 1);
    assert_eq!(meter.segments, [Segment::Off; METER_SEGMENTS]);
}

#[test]
fn the_meter_lights_a_tenth_per_segment_above_the_whole_heat() {
    let meter = HeatMeter::of(3.35);
    assert_eq!(meter.multiplier, 3);
    let lit = meter.segments.iter().filter(|s| **s == Segment::On).count();
    assert_eq!(lit, 4, "0.35 of the level lights the segments at .0 .1 .2 .3");
    assert_eq!(meter.segments[3], Segment::On);
    assert_eq!(meter.segments[4], Segment::Off);
}

#[test]
fn heat_is_clamped_to_one_through_five() {
    assert_eq!(HeatMeter::of(9.0).multiplier, 5);
    assert_eq!(HeatMeter::of(f32::NAN).multiplier, 1);
    assert_eq!(HeatMeter::of(5.0).multiplier, 5);
    assert_eq!(HeatMeter::of(2.0).multiplier, 2);
    assert_eq!(HeatMeter::of(1.99).multiplier, 1);
}

#[test]
fn the_segments_a_part_would_remove_show_as_cooling() {
    let meter = HeatMeter::new(3.5, 3.2);
    assert_eq!(meter.multiplier, 3);
    assert_eq!(meter.segments[1], Segment::On);
    assert_eq!(meter.segments[2], Segment::Cool);
    assert_eq!(meter.segments[4], Segment::Cool);
    assert_eq!(meter.segments[5], Segment::Off);
}
