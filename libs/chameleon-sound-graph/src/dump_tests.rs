//! A printout of every sequence of a car bundle, to read what a car's sound does:
//! `MW2012_GAME_DIR=<2012 install> MW2012_CAR=VEH_1085764_HI cargo test -p chameleon-sound-graph dump_ -- --ignored --nocapture`.

use chameleon_bundle::Bundle;

use crate::{ItemKind, Modulator, SoundGraphs};

fn show(m: &Modulator) -> String {
    format!(
        "[expr={:?} in={:?} out={:?} base={} scale={} pts={:?}]",
        m.expression,
        m.input,
        m.output,
        m.base,
        m.scale,
        m.curve.points.iter().map(|p| (p.x, p.y)).collect::<Vec<_>>()
    )
}

#[test]
#[ignore = "prints; needs the 2012 install"]
fn dump_all_sequences_of_a_car() {
    let (Some(dir), Some(car)) = (std::env::var_os("MW2012_GAME_DIR"), std::env::var_os("MW2012_CAR")) else { return };
    let path = std::path::Path::new(&dir).join("VEHICLES").join(format!("{}.BNDL", car.to_string_lossy()));
    let bytes = std::fs::read(path).unwrap();
    let graphs = SoundGraphs::read(&Bundle::parse(&bytes).unwrap());
    for s in &graphs.sequences {
        println!("SEQ {:x} {} [{}..{}] items {}", s.id & 0xffffff, s.driver, s.lo, s.hi, s.items.len());
        for i in &s.items {
            let head = format!("   [{}..{}] in {:?} out {:?}", i.lo, i.hi, i.fade_in, i.fade_out);
            match &i.kind {
                ItemKind::Ginsu(g) => {
                    let gain = graphs.channel_gains.get(&g.mixer);
                    println!("{head} GINSU {:x} mixer {:x} chgain {gain:?}", g.resource & 0xffffff, g.mixer & 0xffffff)
                }
                ItemKind::Wave(w) => println!(
                    "{head} WAVE {:x?} mixer {:x} chgain {:?} track={} rnd={} gain {} pitch {}",
                    w.waves.iter().map(|x| x & 0xffffff).collect::<Vec<_>>(),
                    w.mixer & 0xffffff,
                    graphs.channel_gains.get(&w.mixer),
                    w.tracks_driver,
                    w.random,
                    show(&w.gain),
                    show(&w.pitch)
                ),
                ItemKind::Mixer(m) => {
                    println!(
                        "{head} MIXER {:x} {:?}",
                        m.channel & 0xffffff,
                        m.gains.iter().map(show).collect::<Vec<_>>()
                    )
                }
                ItemKind::Other(o) => println!("{head} {o}"),
            }
        }
    }
}

#[test]
#[ignore = "prints; needs the 2012 install"]
fn dump_engine_sound_objects() {
    use std::collections::HashSet;
    let (Some(dir), Some(car)) = (std::env::var_os("MW2012_GAME_DIR"), std::env::var_os("MW2012_CAR")) else { return };
    let path = std::path::Path::new(&dir).join("VEHICLES").join(format!("{}.BNDL", car.to_string_lossy()));
    let bytes = std::fs::read(path).unwrap();
    let bundle = Bundle::parse(&bytes).unwrap();
    let debug = bundle.debug_names();
    let name_of = |id: u64| debug.iter().find(|d| d.id.0 == id).map(|d| d.name.clone()).unwrap_or_default();
    let type_kind = chameleon_bundle::ResourceKind::GenesysType.id();
    let type_ids: HashSet<u64> = bundle.entries().iter().filter(|e| e.kind == type_kind).map(|e| e.id).collect();
    let names = crate::graphs::type_names(&bundle, type_kind);
    for (index, entry) in bundle.entries().iter().enumerate() {
        if entry.kind != chameleon_bundle::ResourceKind::GenesysObject.id() {
            continue;
        }
        let Ok(res) = bundle.resource(index) else { continue };
        let class = res
            .imports
            .iter()
            .find(|i| i.offset == 0x8000_0000 && type_ids.contains(&i.id))
            .and_then(|i| names.get(&i.id).cloned())
            .unwrap_or_default();
        println!("OBJ {:x} {} class {} name {}", entry.id & 0xffffff, entry.id, class, name_of(entry.id));
        if class.ends_with("MixerChannel") {
            let gain = res.blocks[0].get(48..52).map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]));
            let parents: Vec<String> = res
                .imports
                .iter()
                .filter(|i| !type_ids.contains(&i.id))
                .map(|i| format!("{:#x}->{:x}", i.offset & 0x7fff_ffff, i.id & 0xffffff))
                .collect();
            println!("    CH {:x} gain {gain:?} parents {parents:?}", entry.id & 0xffffff);
        }
        if !["EngineSound", "EngineSound2", "Vehicle_Sound", "TransmissionWhine"].iter().any(|c| class.ends_with(c)) {
            continue;
        }
        let block = &res.blocks[0];
        for imp in &res.imports {
            if type_ids.contains(&imp.id) {
                continue;
            }
            println!("    handle at {:#x} -> {:x} {}", imp.offset & 0x7fff_ffff, imp.id & 0xffffff, name_of(imp.id));
        }
        println!("    block len {} hex {:02x?}", block.len(), block);
        for imp in &res.imports {
            println!("    import at {:#x} id {:x} type {}", imp.offset, imp.id, type_ids.contains(&imp.id));
        }
    }
}

#[test]
#[ignore = "prints; needs the 2012 install"]
fn survey_the_player_engines_of_all_cars() {
    let Some(dir) = std::env::var_os("MW2012_GAME_DIR") else { return };
    let mut paths: Vec<_> = std::fs::read_dir(std::path::Path::new(&dir).join("VEHICLES"))
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.to_string_lossy().ends_with("_HI.BNDL"))
        .collect();
    paths.sort();
    for path in paths {
        let bytes = std::fs::read(&path).unwrap();
        let graphs = SoundGraphs::read(&Bundle::parse(&bytes).unwrap());
        let name = path.file_stem().unwrap().to_string_lossy().into_owned();
        let Some(player) = graphs.player_engine() else {
            println!("{name}: no player engine group");
            continue;
        };
        let mut parts = Vec::new();
        for s in graphs.sequences_in(player) {
            let kind =
                if s.driver.is_empty() { "-".to_owned() } else { s.driver.rsplit('.').next().unwrap().to_owned() };
            if kind == "-" || kind.contains("DamageBar") {
                continue;
            }
            let ginsu = s.items.iter().filter(|i| matches!(i.kind, ItemKind::Ginsu(_))).count();
            let wave = s.items.iter().filter(|i| matches!(i.kind, ItemKind::Wave(_))).count();
            parts.push(format!("{kind}[{}..{}]g{ginsu}w{wave}", s.lo, s.hi));
        }
        let whine = graphs.group_of("TransmissionWhine").map(|g| graphs.sequences_in(g).count()).unwrap_or(0);
        println!("{name}: {} | whine seqs {whine}", parts.join(" "));
    }
}

#[test]
#[ignore = "prints; needs the 2012 install"]
fn dump_objects_that_read_a_value() {
    let (Some(dir), Some(car), Some(needle)) =
        (std::env::var_os("MW2012_GAME_DIR"), std::env::var_os("MW2012_CAR"), std::env::var("MW2012_NEEDLES").ok())
    else {
        return;
    };
    let path = std::path::Path::new(&dir).join("VEHICLES").join(format!("{}.BNDL", car.to_string_lossy()));
    let bytes = std::fs::read(path).unwrap();
    let bundle = Bundle::parse(&bytes).unwrap();
    for (index, entry) in bundle.entries().iter().enumerate() {
        if entry.kind != chameleon_bundle::ResourceKind::GenesysObject.id() {
            continue;
        }
        let Ok(res) = bundle.resource(index) else { continue };
        let block = &res.blocks[0];
        for n in needle.split(',') {
            if block.windows(n.len()).any(|w| w == n.as_bytes()) {
                println!("{:x} reads {n}", entry.id & 0xffffff);
            }
        }
    }
}

#[test]
#[ignore = "prints; needs the 2012 install"]
fn dump_resource_kinds() {
    let (Some(dir), Some(car)) = (std::env::var_os("MW2012_GAME_DIR"), std::env::var_os("MW2012_CAR")) else { return };
    let path = std::path::Path::new(&dir).join("VEHICLES").join(format!("{}.BNDL", car.to_string_lossy()));
    let bytes = std::fs::read(path).unwrap();
    let bundle = Bundle::parse(&bytes).unwrap();
    let mut kinds = std::collections::BTreeMap::<u32, usize>::new();
    for e in bundle.entries() {
        *kinds.entry(e.kind).or_default() += 1;
    }
    println!("KINDS {kinds:x?}");
}
