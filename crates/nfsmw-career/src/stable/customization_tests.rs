use super::*;
use crate::stable::CarRecord;

#[test]
fn arbitrary_original_bytes_round_trip_and_only_edited_fields_change() {
    let bytes: Vec<_> = (0..CUSTOMIZATION_RECORD_LEN).map(|i| (i * 37) as u8).collect();
    let mut record = CustomizationRecord::parse(&bytes).unwrap();
    assert_eq!(record.as_bytes().as_slice(), bytes);
    assert!(record.set_part(23, Some(1234)));
    record.set_performance([1, 2, 3, 4, 2, 1, 0], 65);
    record.set_tuning([[0.2; 7], [-1.0; 7], [1.0; 7]], 2);
    assert_eq!(record.part(23), Some(1234));
    assert_eq!(record.performance(), [1, 2, 3, 4, 2, 1, 0]);
    assert_eq!(record.junkman(), 65);
    assert_eq!(record.tuning(), [[0.2; 7], [-1.0; 7], [1.0; 7]]);
    assert_eq!(record.active_tuning(), 2);
    for (i, original) in bytes.iter().enumerate() {
        if (46..48).contains(&i) || (280..400).contains(&i) {
            continue;
        }
        assert_eq!(record.as_bytes()[i], *original, "unexpected edit at {i}");
    }
}

#[test]
fn stock_records_use_absent_parts_and_neutral_packages() {
    let record = CustomizationRecord::default();
    assert!((0..139).all(|slot| record.part(slot).is_none()));
    assert_eq!(&record.as_bytes()[..2], &[255, 255]);
    assert_eq!(record.performance(), [0; 7]);
    assert_eq!(record.tuning(), [[0.0; 7]; 3]);
}

#[test]
fn invalid_indices_and_short_records_do_not_panic_or_truncate() {
    let mut record = CustomizationRecord::default();
    assert!(!record.set_part(139, Some(0)));
    assert!(!record.set_part(0, Some(32768)));
    assert!(record.part(usize::MAX).is_none());
    assert!(CustomizationRecord::parse(&[0; 407]).is_err());
}

#[test]
fn updating_a_car_keeps_every_other_record_and_unknown_byte() {
    let mut stable = Stable::empty();
    stable.cars[0] = CarRecord { handle: 42, customization_index: 2, ..CarRecord::free() };
    stable.cars[1] = CarRecord { handle: 43, customization_index: 3, ..CarRecord::free() };
    let before = stable.customization.clone();
    let mut record = stable.customization_for_handle(42).unwrap();
    record.set_part(63, Some(77));
    assert!(stable.set_customization_for_handle(42, &record));
    assert_eq!(stable.customization_for_handle(42).unwrap().part(63), Some(77));
    assert_eq!(stable.customization_for_handle(43).unwrap().part(63), Some(0));
    assert_eq!(&stable.customization[..816], &before[..816]);
    assert_eq!(&stable.customization[1224..], &before[1224..]);
    assert!(!stable.set_customization_for_handle(999, &record));
    stable.cars[0].customization_index = 255;
    assert!(stable.customization_for_handle(42).is_none());
    assert!(!stable.set_customization_for_handle(42, &record));
}
