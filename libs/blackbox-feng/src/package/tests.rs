use super::synth::*;
use super::*;
use crate::hash::fe_hash_upper;

fn sample() -> Vec<u8> {
    let mut group = Obj::group(10, fe_hash_upper("Cluster"));
    group.position = [100.0, 50.0, 20.0];
    let mut image = Obj::image(11, fe_hash_upper("Backing"));
    image.parent = 10;
    image.resource = 0;
    let mut text = Obj::string(12, fe_hash_upper("Digit"), "42");
    text.parent = 10;
    text.label = 0x1234;
    let fade = ScriptSpec {
        id: 0x5b0d9106,
        length: 600,
        flags: 0,
        chain: None,
        tracks: vec![TrackSpec {
            param: 6,
            interp: 1,
            action: 0,
            offset: 0,
            length: 600,
            keys: vec![(-1, vec![255, 255, 255, 255]), (0, vec![0, 0, 0, (-255i32) as u32]), (600, vec![0, 0, 0, 0])],
        }],
        events: vec![(0xBEEF, 0xFFFF_FFFF, 600)],
    };
    image.scripts.push(fade.build());
    image.responses = responses(&[(0x77, vec![(0, 0x5b0d9106, 0), (2, 0x99, 0xFFFF_FFFF)])]);
    package(
        "Test.fng",
        &[("Outrun_Backing.tga", 1), ("FONT_MW_BODY.ffn", 2)],
        &[group, image, text],
        &responses(&[(0x55, vec![(1, 0x66, 0)])]),
        &[(0x88, vec![11, 12])],
    )
}

#[test]
fn parses_header_resources_and_objects() {
    let p = Package::parse(&sample()).unwrap();
    assert_eq!(p.name, "Test.fng");
    assert_eq!(p.file_name, "Global\\Test.fng");
    assert_eq!(p.version, 0x20000);
    assert_eq!(p.resources.len(), 2);
    assert_eq!(p.resources[0].kind, ResourceKind::Image);
    assert_eq!(p.resources[0].handle, fe_hash_upper("OUTRUN_BACKING"));
    assert_eq!(p.resources[1].kind, ResourceKind::Font);
    assert_eq!(p.objects.len(), 3);
    let group = &p.objects[0];
    assert_eq!((group.kind, group.guid, group.parent), (ObjectKind::Group, 10, None));
    assert_eq!(group.data.position().to_array(), [100.0, 50.0, 20.0]);
    let image = &p.objects[1];
    assert_eq!((image.kind, image.parent, image.resource), (ObjectKind::Image, Some(10), Some(0)));
    assert_eq!(image.data.uv(), [0.0, 0.0, 1.0, 1.0]);
    let text = &p.objects[2];
    assert_eq!(text.string.as_ref().unwrap().text, "42");
    assert_eq!(text.string.as_ref().unwrap().label, 0x1234);
}

#[test]
fn parses_scripts_with_base_and_delta_keys() {
    let p = Package::parse(&sample()).unwrap();
    let s = &p.objects[1].scripts[0];
    assert_eq!((s.id, s.length, s.end_behaviour()), (0x5b0d9106, 600, 0));
    assert_eq!(s.events.len(), 1);
    assert_eq!(s.events[0].time, 600);
    let t = &s.tracks[0];
    assert_eq!((t.param, t.interp, t.offset), (ParamType::Colour, Interp::Linear, 0));
    assert_eq!(t.base.value, [255, 255, 255, 255]);
    assert_eq!(t.keys.len(), 2);
    assert_eq!(t.keys[0].value[3] as i32, -255);
}

#[test]
fn parses_responses_and_targets() {
    let p = Package::parse(&sample()).unwrap();
    let r = &p.objects[1].responses[0];
    assert_eq!(r.message, 0x77);
    assert_eq!(r.responses[0].kind, ResponseKind::SetScript);
    assert_eq!(r.responses[0].param.number(), 0x5b0d9106);
    assert_eq!(r.responses[1].kind, ResponseKind::PostToGame);
    assert_eq!(r.responses[1].target, 0xFFFF_FFFF);
    assert_eq!(p.responses_to(0x55).unwrap().responses[0].kind, ResponseKind::PostToFEng);
    assert_eq!(p.targets_of(0x88), &[11, 12]);
    assert!(p.targets_of(0x01).is_empty());
}

#[test]
fn lookups() {
    let p = Package::parse(&sample()).unwrap();
    assert_eq!(p.find_by_hash(fe_hash_upper("backing")), Some(1));
    assert_eq!(p.find_by_guid(12), Some(2));
    assert_eq!(p.find_by_guid(99), None);
}

#[test]
fn rejects_garbage_without_panicking() {
    assert!(Package::parse(&[]).is_err());
    assert!(Package::parse(&[1, 2, 3, 4, 5, 6, 7, 8, 9]).is_err());
    let good = sample();
    for cut in [10, 40, 100, good.len() / 2, good.len() - 3] {
        let _ = Package::parse(&good[..cut]);
    }
    // Flip bytes everywhere: it may fail, it must not panic.
    for i in (0..good.len()).step_by(7) {
        let mut bad = good.clone();
        bad[i] ^= 0xFF;
        let _ = Package::parse(&bad);
    }
}

#[test]
fn old_versions_are_refused() {
    let mut p = sample();
    // The version word is the first word of the PkHd payload: find "PkHd" and patch.
    let at = p.windows(4).position(|w| w == b"PkHd").unwrap() + 8;
    p[at..at + 4].copy_from_slice(&0x10000u32.to_le_bytes());
    assert!(matches!(Package::parse(&p), Err(Error::OldVersion(0x10000))));
}
