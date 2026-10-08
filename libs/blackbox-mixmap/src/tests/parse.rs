use super::*;
use crate::{Error, Event, MapBuilder, MixMap, Spatial, SpatialRecord};

fn rich_state() -> State {
    State {
        controls: vec![
            control(controller(8, 0, 0, 1), 0xD8F0, vec![]),
            control(controller(4, 0, 0, 2), 0x0200, vec![control_ref(0, 0), sub_ref(0, 0)]),
        ],
        events: vec![Event {
            id: SourceId(0xA201_3000),
            swing: -500,
            trigger: object_input(0, 3, 7),
            params: [0x8006, 0, 0x8006],
            scales: vec![control_ref(0, 1)],
        }],
        spatial: vec![Spatial {
            id: SourceId(0x8100_0070),
            records: vec![SpatialRecord { info: 0x9000_0000, curves: 0x4444_0154, ranges: [0x0046_0004; 4] }],
        }],
        subs: vec![sub(vec![control_ref(0, 0), event_ref(0, 0)], -5000, 7000)],
        masters: vec![MasterChannel {
            object: SourceId(0x4000_0010),
            base: -400,
            inputs: vec![spatial_ref(0, 0), sub_ref(0, 0)],
            kind: OutputKind::Pitch,
            words: vec![
                PresetWord { slot: 4, spatial: 0, azimuth: false, offset: -35 },
                PresetWord { slot: 0, spatial: 0, azimuth: true, offset: 0 },
            ],
        }],
    }
}

#[test]
fn a_built_map_parses_back_to_what_was_built() {
    let bytes = MapBuilder::new(3).state(0, rich_state()).state(2, State::default()).build();
    let map = MixMap::parse(&bytes).unwrap();
    assert_eq!(map.states.len(), 3);
    assert_eq!(map.map_type, 0);
    assert_eq!(map.state(0), Some(&rich_state()));
    assert_eq!(map.state(1), None);
    assert_eq!(map.state(2), Some(&State::default()));
    assert_eq!(map.state(9), None);
}

#[test]
fn a_control_reports_its_offset_and_depth() {
    let boost = control(controller(0, 0, 0, 0), 0x0064, vec![]);
    assert_eq!(boost.offset_and_depth(), (100, -100));
    assert_eq!(boost.shape(), 0);
    let cut = control(controller(9, 0, 0, 0), 0xD8F0, vec![]);
    assert_eq!(cut.offset_and_depth(), (0, -10000));
    assert_eq!(cut.shape(), 9);
}

#[test]
fn truncated_data_is_an_error() {
    let bytes = MapBuilder::new(1).state(0, rich_state()).build();
    for len in [0, 3, 8, 15, 20, 40, bytes.len() - 1] {
        assert!(MixMap::parse(&bytes[..len]).is_err(), "{len} bytes parsed");
    }
}

#[test]
fn bad_offsets_and_counts_are_errors() {
    let mut bytes = MapBuilder::new(1).state(0, rich_state()).build();
    // The state table entry points past the end.
    let mut wrong = bytes.clone();
    wrong[16..20].copy_from_slice(&(bytes.len() as u32 + 100).to_le_bytes());
    assert!(matches!(MixMap::parse(&wrong), Err(Error::BadOffset(_))));
    // A huge state count.
    bytes[4..8].copy_from_slice(&0x00FF_FFFFu32.to_le_bytes());
    assert!(matches!(MixMap::parse(&bytes), Err(Error::BadCount(_))));
}

#[test]
fn garbage_never_panics() {
    let mut seed = 12345u32;
    for len in [16usize, 64, 200, 1000] {
        for _ in 0..50 {
            let bytes: Vec<u8> = (0..len)
                .map(|_| {
                    seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                    (seed >> 24) as u8
                })
                .collect();
            let _ = MixMap::parse(&bytes);
        }
    }
}

#[test]
fn the_envelope_kind_comes_from_the_id() {
    let kind =
        |id: u32| Event { id: SourceId(id), swing: 0, trigger: SourceId(0), params: [0; 3], scales: vec![] }.kind();
    assert_eq!(kind(0xA000_3000), crate::EnvelopeKind::Ar);
    assert_eq!(kind(0xA100_3000), crate::EnvelopeKind::Asr);
    assert_eq!(kind(0xA200_3000), crate::EnvelopeKind::Atr);
    assert_eq!(kind(0xA300_3000), crate::EnvelopeKind::Lfo);
}
