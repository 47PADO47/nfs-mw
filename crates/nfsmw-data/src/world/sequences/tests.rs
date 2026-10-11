use super::*;

fn event(id: u32, words: &[u32]) -> Event {
    let mut data = id.to_le_bytes().to_vec();
    for word in words {
        data.extend(word.to_le_bytes());
    }
    Event { id, data }
}

#[test]
fn breaker_parameters_and_visibility_events_are_distinct() {
    let e = event(0x7624_42F3, &[1, 2, 3, 30.0_f32.to_bits()]);
    assert_eq!(
        WorldEvent::decode(&e).unwrap(),
        WorldEvent::AreaStimulus { system: 1, stimulus: 2, queue_mode: 3, radius: 30.0 }
    );
    assert_eq!(
        WorldEvent::decode(&event(0xDDA7_829B, &[16.0_f32.to_bits(), 5.0_f32.to_bits()])).unwrap(),
        WorldEvent::StopCops { radius: 16.0, duration: 5.0 }
    );
    assert_eq!(WorldEvent::decode(&event(0x0713_D2FA, &[])).unwrap(), WorldEvent::HideObject);
    assert_eq!(WorldEvent::decode(&event(0xBD2F_6FF1, &[])).unwrap(), WorldEvent::KillObject);
    assert_eq!(WorldEvent::decode(&event(0xEFFE_CDD6, &[5])).unwrap(), WorldEvent::HidePart(5));
    assert_eq!(WorldEvent::decode(&event(0x7DF7_F613, &[5])).unwrap(), WorldEvent::Other(0x7DF7_F613));
}

#[test]
fn truncated_and_nonfinite_area_parameters_are_rejected() {
    assert!(WorldEvent::decode(&event(0x7624_42F3, &[1, 2, 3])).is_err());
    assert!(WorldEvent::decode(&event(0x7624_42F3, &[1, 2, 3, f32::NAN.to_bits()])).is_err());
    assert!(WorldEvent::decode(&event(0xDDA7_829B, &[1])).is_err());
}
