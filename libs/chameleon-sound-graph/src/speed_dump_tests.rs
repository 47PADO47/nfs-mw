//! Scratch printout of the objects of one bundle by relative path:
//! `MW2012_GAME_DIR=<2012 install> MW2012_BUNDLE=SOUND/SPEEDSOUNDS.BNDL cargo test -p chameleon-sound-graph speed_dump -- --ignored --nocapture`.

use chameleon_bundle::Bundle;

#[test]
#[ignore = "prints; needs the 2012 install"]
fn speed_dump_objects() {
    let (Some(dir), Some(rel)) = (std::env::var_os("MW2012_GAME_DIR"), std::env::var_os("MW2012_BUNDLE")) else {
        return;
    };
    let bytes = std::fs::read(std::path::Path::new(&dir).join(rel)).unwrap();
    let bundle = Bundle::parse(&bytes).unwrap();
    let debug = bundle.debug_names();
    let name_of = |id: u64| debug.iter().find(|d| d.id.0 == id).map(|d| d.name.clone()).unwrap_or_default();
    let type_kind = chameleon_bundle::ResourceKind::GenesysType.id();
    let type_ids: std::collections::HashSet<u64> =
        bundle.entries().iter().filter(|e| e.kind == type_kind).map(|e| e.id).collect();
    let names = crate::graphs::type_names(&bundle, type_kind);
    let filter = std::env::var("MW2012_CLASS").ok();
    for (index, entry) in bundle.entries().iter().enumerate() {
        let Ok(res) = bundle.resource(index) else {
            println!("RES {:x} kind {:x} unreadable", entry.id & 0xffffff, entry.kind);
            continue;
        };
        let class = res
            .imports
            .iter()
            .find(|i| i.offset == 0x8000_0000 && type_ids.contains(&i.id))
            .and_then(|i| names.get(&i.id).cloned())
            .unwrap_or_default();
        println!(
            "RES {:x} kind {:x} class {} name {} blocks {}",
            entry.id & 0xffffff,
            entry.kind,
            class,
            name_of(entry.id),
            res.blocks.len()
        );
        if filter.as_ref().is_some_and(|f| !class.contains(f.as_str())) {
            continue;
        }
        if entry.kind == 0x81 {
            let main = &res.blocks[0];
            println!(
                "  SAMPLE blocks {} len {} head {:02x?}",
                res.blocks.len(),
                main.len(),
                &main[..main.len().min(176)]
            );
            if let (Some(out), Some(size)) = (std::env::var_os("MW2012_OUT"), main.get(24..28)) {
                let size = u32::from_le_bytes(size.try_into().unwrap()) as usize;
                if let Some(mp3) = main.get(128..128 + size).and_then(|s| ea_audio::sps::to_mp3(s).ok()) {
                    for (n, core) in mp3.cores.iter().enumerate() {
                        let path = std::path::Path::new(&out).join(format!("{:x}_{n}.mp3", entry.id & 0xffffff));
                        std::fs::write(path, &core.bytes).unwrap();
                    }
                }
            }
            for imp in &res.imports {
                println!("     sample import at {:#x} id {:x}", imp.offset, imp.id & 0xffffff);
            }
        }
        if entry.kind == type_kind && std::env::var_os("MW2012_TYPES").is_some() {
            let main = &res.blocks[0];
            let tname = names.get(&entry.id).cloned().unwrap_or_default();
            let end = u32::from_le_bytes(main[0x20..0x24].try_into().unwrap()) as usize;
            println!("  TYPE {tname} size {}", u32::from_le_bytes(main[0x18..0x1c].try_into().unwrap()));
            let mut at = 0x28;
            while at + 28 <= main.len() && at < end {
                let w = |o: usize| u32::from_le_bytes(main[at + o..at + o + 4].try_into().unwrap());
                println!(
                    "     field type {:08x} name {:08x} size {} offset {} count {} align {}",
                    w(0),
                    w(8),
                    w(12),
                    w(16),
                    u16::from_le_bytes([main[at + 20], main[at + 21]]),
                    u16::from_le_bytes([main[at + 22], main[at + 23]])
                );
                at += 28;
            }
        }
        if entry.kind != chameleon_bundle::ResourceKind::GenesysObject.id() {
            continue;
        }
        let block = &res.blocks[0];
        println!("    block len {} hex {:02x?}", block.len(), block);
        for imp in &res.imports {
            let t = type_ids.contains(&imp.id);
            println!(
                "    import at {:#x} id {:x} type {} {}",
                imp.offset,
                imp.id & 0xffffff,
                t,
                if t { names.get(&imp.id).cloned().unwrap_or_default() } else { String::new() }
            );
        }
    }
}

#[test]
#[ignore = "prints; needs the 2012 install"]
fn speed_scan_all_bundles_for_a_class() {
    let (Some(dir), Some(needle)) = (std::env::var_os("MW2012_GAME_DIR"), std::env::var("MW2012_CLASS").ok()) else {
        return;
    };
    let mut stack = vec![std::path::PathBuf::from(dir)];
    while let Some(path) = stack.pop() {
        let Ok(read) = std::fs::read_dir(&path) else { continue };
        for entry in read.flatten() {
            let p = entry.path();
            if p.is_dir() {
                stack.push(p);
                continue;
            }
            if !p.to_string_lossy().to_ascii_uppercase().ends_with(".BNDL") {
                continue;
            }
            let Ok(bytes) = std::fs::read(&p) else { continue };
            let Ok(bundle) = Bundle::parse(&bytes) else { continue };
            let type_kind = chameleon_bundle::ResourceKind::GenesysType.id();
            let names = crate::graphs::type_names(&bundle, type_kind);
            let ids: Vec<u64> = names.iter().filter(|(_, n)| n.contains(needle.as_str())).map(|(i, _)| *i).collect();
            if ids.is_empty() {
                continue;
            }
            let mut count = 0;
            for (index, e) in bundle.entries().iter().enumerate() {
                if e.kind != chameleon_bundle::ResourceKind::GenesysObject.id() {
                    continue;
                }
                let Ok(res) = bundle.resource(index) else { continue };
                if res.imports.iter().any(|i| i.offset == 0x8000_0000 && ids.contains(&i.id)) {
                    count += 1;
                    println!("  OBJ {:x}", e.id & 0xffffff);
                }
            }
            println!("{}: type present, {count} objects", p.display());
        }
    }
}

/// Class histogram and the readable strings of the objects whose block contains `MW2012_NEEDLES` (comma separated, any case).
#[test]
#[ignore = "prints; needs the 2012 install"]
fn speed_strings_of_a_bundle() {
    let (Some(dir), Some(rel)) = (std::env::var_os("MW2012_GAME_DIR"), std::env::var_os("MW2012_BUNDLE")) else {
        return;
    };
    let bytes = std::fs::read(std::path::Path::new(&dir).join(rel)).unwrap();
    let bundle = Bundle::parse(&bytes).unwrap();
    let type_kind = chameleon_bundle::ResourceKind::GenesysType.id();
    let type_ids: std::collections::HashSet<u64> =
        bundle.entries().iter().filter(|e| e.kind == type_kind).map(|e| e.id).collect();
    let names = crate::graphs::type_names(&bundle, type_kind);
    let needles: Vec<String> =
        std::env::var("MW2012_NEEDLES").unwrap_or_default().split(',').map(|s| s.to_ascii_lowercase()).collect();
    let mut hist = std::collections::BTreeMap::<String, usize>::new();
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
        *hist.entry(class.clone()).or_default() += 1;
        let mut strings = Vec::new();
        for block in &res.blocks {
            let mut cur = Vec::new();
            for &b in block.iter().chain(std::iter::once(&0u8)) {
                if (0x20..0x7f).contains(&b) {
                    cur.push(b);
                    continue;
                }
                if cur.len() >= 5 {
                    strings.push(String::from_utf8_lossy(&cur).into_owned());
                }
                cur.clear();
            }
        }
        let hit = strings
            .iter()
            .any(|s| needles.iter().any(|n| !n.is_empty() && s.to_ascii_lowercase().contains(n.as_str())));
        if hit {
            let handles: Vec<String> = res
                .imports
                .iter()
                .filter(|i| !type_ids.contains(&i.id))
                .map(|i| format!("{:x}", i.id & 0xffffff))
                .collect();
            println!("OBJ {:x} class {class} strings {strings:?} handles {handles:?}", entry.id & 0xffffff);
        }
    }
    println!("CLASSES {hist:#?}");
}

fn show(m: &crate::Modulator) -> String {
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

/// Sequences of a bundle (by relative path) whose driver contains `MW2012_DRIVER`.
#[test]
#[ignore = "prints; needs the 2012 install"]
fn speed_sequences_of_a_bundle() {
    use crate::{ItemKind, SoundGraphs};
    let (Some(dir), Some(rel), Ok(driver)) =
        (std::env::var_os("MW2012_GAME_DIR"), std::env::var_os("MW2012_BUNDLE"), std::env::var("MW2012_DRIVER"))
    else {
        return;
    };
    let bytes = std::fs::read(std::path::Path::new(&dir).join(rel)).unwrap();
    let graphs = SoundGraphs::read(&Bundle::parse(&bytes).unwrap());
    for s in graphs.sequences.iter().filter(|s| s.driver.contains(driver.as_str())) {
        println!("SEQ {:x} {} [{}..{}] items {}", s.id & 0xffffff, s.driver, s.lo, s.hi, s.items.len());
        for i in &s.items {
            let head = format!("   [{}..{}] in {:?} out {:?}", i.lo, i.hi, i.fade_in, i.fade_out);
            match &i.kind {
                ItemKind::Ginsu(g) => {
                    println!("{head} GINSU {:x} mixer {:x}", g.resource & 0xffffff, g.mixer & 0xffffff)
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
                ItemKind::Mixer(m) => println!(
                    "{head} MIXER {:x} chgain {:?} {:?}",
                    m.channel & 0xffffff,
                    graphs.channel_gains.get(&m.channel),
                    m.gains.iter().map(show).collect::<Vec<_>>()
                ),
                ItemKind::Other(o) => println!("{head} {o}"),
            }
        }
    }
}

/// Header of every sample of a bundle whose id (low 24 bits, hex) is in `MW2012_IDS` (comma separated, all when unset);
/// writes the decoded-to-MP3 cores to `MW2012_OUT` when set.
#[test]
#[ignore = "prints; needs the 2012 install"]
fn speed_samples_of_a_bundle() {
    let (Some(dir), Some(rel)) = (std::env::var_os("MW2012_GAME_DIR"), std::env::var_os("MW2012_BUNDLE")) else {
        return;
    };
    let bytes = std::fs::read(std::path::Path::new(&dir).join(rel)).unwrap();
    let bundle = Bundle::parse(&bytes).unwrap();
    let ids: Vec<u64> = std::env::var("MW2012_IDS")
        .map(|s| s.split(',').filter_map(|x| u64::from_str_radix(x, 16).ok()).collect())
        .unwrap_or_default();
    for (index, entry) in bundle.entries().iter().enumerate() {
        if entry.kind != 0x81 || (!ids.is_empty() && !ids.contains(&(entry.id & 0xffffff))) {
            continue;
        }
        let Ok(res) = bundle.resource(index) else { continue };
        let main = &res.blocks[0];
        let size = u32::from_le_bytes(main[24..28].try_into().unwrap()) as usize;
        let ms = f32::from_le_bytes(main[20..24].try_into().unwrap());
        let vol = f32::from_le_bytes(main[16..20].try_into().unwrap());
        let Some(stream) = main.get(128..128 + size) else { continue };
        let Ok(h) = ea_audio::sps::read_header(stream) else {
            println!("SAMPLE {:x} unreadable header", entry.id & 0xffffff);
            continue;
        };
        println!(
            "SAMPLE {:x} vol {vol} {ms} ms ch {} rate {} samples {} loop {:?}",
            entry.id & 0xffffff,
            h.channels,
            h.sample_rate,
            h.samples,
            h.loop_start
        );
        let Some(out) = std::env::var_os("MW2012_OUT") else { continue };
        let Ok(mp3) = ea_audio::sps::to_mp3(stream) else { continue };
        for (n, core) in mp3.cores.iter().enumerate() {
            let path = std::path::Path::new(&out).join(format!("{:x}_{n}.mp3", entry.id & 0xffffff));
            std::fs::write(path, &core.bytes).unwrap();
        }
    }
}
