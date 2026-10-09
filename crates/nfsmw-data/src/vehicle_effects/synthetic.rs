//! Synthetic attribute graphs built with the existing generic vault fixture helper.
use super::fixture::{Data, Def, Entry, VaultBuilder, add_class, add_collection, pack};
use blackbox_attrib::{Database, vlt_hash};

const REF: &str = "Attrib::RefSpec";
const FLOAT: &str = "EA::Reflection::Float";
const VECTOR: &str = "Attrib::Types::Vector4";
type Field = (&'static str, &'static str, u16, bool);

fn class(b: &mut VaultBuilder, name: &str, fields: &[Field]) {
    let defs = fields
        .iter()
        .map(|&(name, type_name, size, array)| Def {
            name,
            type_name,
            offset: 0,
            size,
            max_count: 64,
            flags: 4 | u8::from(array),
            align_log2: 2,
        })
        .collect::<Vec<_>>();
    add_class(b, name, &defs, 0);
}

fn collection(b: &mut VaultBuilder, class: &str, name: &str, parent: Option<&str>, fields: &[(Field, Vec<u8>)]) {
    let types = fields.iter().map(|((_, kind, _, _), _)| *kind).collect::<Vec<_>>();
    let entries = fields
        .iter()
        .enumerate()
        .map(|(i, ((field, _, size, array), bytes))| {
            // Legacy scalar values up to four bytes are stored in the entry's data word,
            // regardless of pointer fixups; arrays and larger values are indirect.
            let data = match (*array, *size) {
                (false, 4) => Data::Inline(u32::from_le_bytes(bytes.as_slice().try_into().unwrap())),
                _ => Data::Ptr(b.bin_data(bytes)),
            };
            Entry { field, type_index: i as u16, node_flags: u8::from(*array) * 2, data }
        })
        .collect::<Vec<_>>();
    add_collection(b, class, name, parent, None, &types, &entries);
}

fn reference(class: &str, name: &str) -> Vec<u8> {
    [vlt_hash(class), vlt_hash(name), 0].iter().flat_map(|v| v.to_le_bytes()).collect()
}

fn array(size: u16, items: &[Vec<u8>]) -> Vec<u8> {
    [items.len() as u16, items.len() as u16, size, 0]
        .iter()
        .flat_map(|v| v.to_le_bytes())
        .chain(items.iter().flatten().copied())
        .collect()
}

fn record(surface: &str, effect: &str, min: f32, max: f32, bad: &str) -> Vec<u8> {
    let selector_class = match bad {
        "selector" => "effects",
        _ => "simsurface",
    };
    let effect_class = match bad {
        "effect" => "simsurface",
        _ => "effects",
    };
    let mut bytes = reference(selector_class, surface);
    bytes.extend(reference(effect_class, effect));
    bytes.extend(min.to_le_bytes());
    bytes.extend(max.to_le_bytes());
    bytes
}

pub fn database(bad: &str) -> Database {
    let mut b = VaultBuilder::new("visual-fixture", &[]);
    let link_type = match bad {
        "record_type" => "OtherRecord",
        _ => "EffectLinkageRecord",
    };
    let link = |name| (name, link_type, 32, true);
    let group = ("emittergroup", REF, 12, false);
    let inherit = ("InheritVelocity", FLOAT, 4, false);
    let emitters = ("Emitters", REF, 12, true);
    let xenon = ("XenonEffect", REF, 12, true);
    let ng = ("NGEmitter", REF, 12, true);
    let style = [("Colour1", VECTOR, 16, false), ("Life", FLOAT, 4, false), ("LifeVariance", FLOAT, 4, false)];
    let motion = [
        ("VolumeCenter", VECTOR, 16, false),
        ("VolumeExtent", VECTOR, 16, false),
        ("VelocityStart", VECTOR, 16, false),
        ("VelocityDelta", VECTOR, 16, false),
        ("VelocityInherit", VECTOR, 16, false),
        ("GravityStart", FLOAT, 4, false),
        ("GravityDelta", FLOAT, 4, false),
        ("NumParticles", FLOAT, 4, false),
        ("NumParticlesVariance", FLOAT, 4, false),
        ("LengthStart", FLOAT, 4, false),
        ("LengthDelta", FLOAT, 4, false),
        ("HeightStart", FLOAT, 4, false),
    ];
    class(&mut b, "pvehicle", &[link("OnHitWorld"), link("OnScrapeWorld")]);
    class(&mut b, "simsurface", &[]);
    class(&mut b, "effects", &[group, inherit]);
    class(&mut b, "emittergroup", &[emitters]);
    class(&mut b, "emitterdata", &[xenon]);
    class(&mut b, "fuelcell_effect", &[ng]);
    class(&mut b, "fuelcell_emitter", &[style.as_slice(), motion.as_slice()].concat());
    for (name, parent) in [
        ("default", None),
        ("null", None),
        ("unknown", Some("default")),
        ("solid", Some("default")),
        ("asphalt", Some("solid")),
        ("wood", Some("solid")),
        ("wood_child", Some("wood")),
    ] {
        collection(&mut b, "simsurface", name, parent, &[]);
    }
    let (min, max) = match bad {
        "range" => (3.0, 2.0),
        "nan_range" => (f32::NAN, 30.0),
        _ => (1.0, 30.0),
    };
    let mut hit = vec![
        record("default", "spark", min, max, bad),
        record("wood", "unsupported", 0.5, 30.0, ""),
        record("null", "unsupported", 0.5, 30.0, ""),
    ];
    if bad == "unknown_override" {
        hit.push(record("unknown", "unsupported", 0.5, 30.0, ""));
    }
    let hit = array(32, &hit);
    let scrape = array(32, &[record("default", "spark", 5.0, 30.0, bad), record("wood", "unsupported", 5.0, 30.0, "")]);
    collection(&mut b, "pvehicle", "default", None, &[(link("OnHitWorld"), hit), (link("OnScrapeWorld"), scrape)]);
    collection(&mut b, "pvehicle", "synthetic_car", Some("default"), &[]);
    let group_class = match bad {
        "group" => "effects",
        _ => "emittergroup",
    };
    collection(
        &mut b,
        "effects",
        "spark",
        None,
        &[(group, reference(group_class, "spark_group")), (inherit, 1.0_f32.to_le_bytes().to_vec())],
    );
    collection(&mut b, "effects", "unsupported", None, &[(group, reference("emittergroup", "empty"))]);
    let emitter_class = match bad {
        "emitter" => "fuelcell_emitter",
        _ => "emitterdata",
    };
    collection(
        &mut b,
        "emittergroup",
        "spark_group",
        None,
        &[(emitters, array(12, &[reference(emitter_class, "conventional")]))],
    );
    collection(&mut b, "emittergroup", "empty", None, &[(emitters, array(12, &[]))]);
    let xenon_class = match bad {
        "xenon" => "emittergroup",
        _ => "fuelcell_effect",
    };
    collection(
        &mut b,
        "emitterdata",
        "conventional",
        None,
        &[(xenon, array(12, &[reference(xenon_class, "fxsprk_line")]))],
    );
    if bad != "missing" {
        let line_class = match bad {
            "line" => "emitterdata",
            _ => "fuelcell_emitter",
        };
        collection(
            &mut b,
            "fuelcell_effect",
            "fxsprk_line",
            None,
            &[(ng, array(12, &[reference(line_class, "emsprk_line2"), reference(line_class, "emsprk_line1")]))],
        );
    }
    collection(
        &mut b,
        "fuelcell_effect",
        "contrail",
        None,
        &[(ng, array(12, &[reference("fuelcell_emitter", "trail3")]))],
    );
    let color = match bad {
        "color" => [f32::NAN, 0.2, 0.3, 0.4],
        _ => [0.1, 0.2, 0.3, 0.4],
    };
    let life = match bad {
        "life" => 0.0_f32,
        _ => 2.0,
    };
    let variance = match bad {
        "variance" => 3.0_f32,
        _ => 0.1,
    };
    for name in ["emsprk_line1", "emsprk_line2", "trail3"] {
        let mut fields = vec![
            (style[0], color.iter().flat_map(|v| v.to_le_bytes()).collect()),
            (style[1], life.to_le_bytes().to_vec()),
            (style[2], variance.to_le_bytes().to_vec()),
        ];
        for field in motion {
            let bytes = match field.1 {
                VECTOR => [0.0_f32; 4].iter().flat_map(|v| v.to_le_bytes()).collect(),
                _ => 0.0_f32.to_le_bytes().to_vec(),
            };
            fields.push((field, bytes));
        }
        collection(&mut b, "fuelcell_emitter", name, None, &fields);
    }
    Database::open(&pack(&[b.finish()])).unwrap()
}
