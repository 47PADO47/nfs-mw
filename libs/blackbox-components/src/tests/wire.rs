//! The byte form: it round-trips, it is canonical, and it refuses what another machine could send.

use crate::{Input, Pick, Selection, WIRE_VERSION, WireError};

fn sample() -> Selection {
    let mut selection = Selection::default();
    selection.set(
        "paint",
        Pick::of("paint.metallic").with("colour", Input::Rgb([1, 2, 3])).with("amount", Input::Num(0.25)),
    );
    selection.set("plate", Pick::of("plate.text").with("text", Input::Text("NEED4SPD".into())));
    selection.set("bar_pad", Pick::empty());
    selection
}

#[test]
fn a_selection_round_trips_and_has_one_encoding() {
    let selection = sample();
    let bytes = selection.encode();
    assert_eq!(Selection::decode(&bytes).unwrap(), selection);
    assert_eq!(Selection::decode(&bytes).unwrap().encode(), bytes);
    assert_eq!(Selection::default().encode(), [WIRE_VERSION, 0, 0]);
}

#[test]
fn the_json_form_round_trips_too() {
    let selection = sample();
    let text = serde_json::to_string(&selection).unwrap();
    assert_eq!(serde_json::from_str::<Selection>(&text).unwrap(), selection);
}

#[test]
fn every_truncation_is_refused() {
    let bytes = sample().encode();
    for end in 0..bytes.len() {
        assert!(Selection::decode(&bytes[..end]).is_err(), "{end}");
    }
}

#[test]
fn garbage_is_refused_with_a_reason() {
    let good = sample().encode();
    let mut bad_version = good.clone();
    bad_version[0] = 9;
    assert_eq!(Selection::decode(&bad_version), Err(WireError::Version(9)));

    let mut trailing = good.clone();
    trailing.push(0);
    assert_eq!(Selection::decode(&trailing), Err(WireError::Trailing));

    // Too many picks for the header, before any is read.
    assert_eq!(Selection::decode(&[WIRE_VERSION, 0xff, 0xff]), Err(WireError::TooLarge));

    // Two picks in the wrong order: "b" then "a".
    let mut unordered = vec![WIRE_VERSION, 2, 0];
    for slot in ["b", "a"] {
        unordered.extend([1, 0, slot.as_bytes()[0], 0, 0]);
    }
    assert_eq!(Selection::decode(&unordered), Err(WireError::Order));

    // A string that is not UTF-8, and a number that is not finite.
    assert_eq!(Selection::decode(&[WIRE_VERSION, 1, 0, 1, 0, 0xff, 0, 0]), Err(WireError::Utf8));
    let mut nan = vec![WIRE_VERSION, 1, 0, 1, 0, b's', 0, 1, 1, 0, b'k', 1];
    nan.extend(f64::NAN.to_bits().to_le_bytes());
    assert_eq!(Selection::decode(&nan), Err(WireError::NotFinite));
}

#[test]
fn no_input_makes_the_decoder_panic() {
    // A cheap sweep of short, odd inputs: the decoder returns a result for every one of them.
    for a in 0..=255u8 {
        for b in [0u8, 1, 2, 255] {
            let _ = Selection::decode(&[WIRE_VERSION, a, b, a, b, a, b, a, b]);
            let _ = Selection::decode(&[a, b, a]);
        }
    }
}
