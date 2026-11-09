//! The numeric audit of the replaced trees against the installed world (`docs/formats/chameleon-vegetation.md`, "Standing on the
//! ground"): every copy a tree set makes, as it is without settling (the way the set was) and with it (the way it is now), is judged
//! against the terrain under it, the roads next to it and the objects of its tile ([`judge`]): does its trunk float over the ground or sink
//! under it, does it stand on a carriageway, does its crown hang low over one. Needs the 2005 install and the converted pack; writes a summary
//! and a tab-separated list of the copies that are wrong. Not part of the normal run.

use std::collections::{BTreeMap, HashMap};
use std::fmt::Write as _;
use std::path::PathBuf;

use blackbox_hash::bstring_hash;
use blackbox_modding::content::{FitLimits, SwapFit, parse_model_swap_profile};
use game_install::GameDir;
use nfsmw_data::world::{DEFAULT_TRACK, WorldIndex, parse_section};

use super::active::world_copy_base;
use super::audit::{Verdict, judge};
use super::clear::Neighbours;
use super::fit::{Frame, Prepared, plan_copies};
use super::ground::Terrain;
use super::place::Placement;
use super::roads::Roads;
use super::settle::{Settled, Site};

fn env_path(name: &str) -> PathBuf {
    PathBuf::from(std::env::var_os(name).unwrap_or_else(|| panic!("set {name}")))
}

/// How many copies, and how many of them each check flags.
#[derive(Debug, Default, Clone, Copy)]
struct Counts {
    copies: usize,
    floating: usize,
    sunk: usize,
    on_road: usize,
    low_crown: usize,
    any: usize,
}

impl Counts {
    fn add(&mut self, verdict: &Verdict) {
        self.copies += 1;
        self.floating += usize::from(verdict.floating);
        self.sunk += usize::from(verdict.sunk);
        self.on_road += usize::from(verdict.on_road);
        self.low_crown += usize::from(verdict.low_crown);
        self.any += usize::from(verdict.any());
    }

    fn line(&self) -> String {
        format!(
            "copies {} flagged {} (floating {}, sunk {}, on the road {}, low crown {})",
            self.copies, self.any, self.floating, self.sunk, self.on_road, self.low_crown
        )
    }
}

/// What one pass (settling off, then on) found: totals, by family, and the flagged copies.
#[derive(Default)]
struct Pass {
    world: Counts,
    local: Counts,
    dropped: usize,
    settled: Settled,
    by_family: BTreeMap<String, Counts>,
    rows: Vec<String>,
}

struct Swapped {
    model: Prepared,
    fit: SwapFit,
}

fn flags(verdict: &Verdict) -> String {
    let named = [
        (verdict.floating, "floating"),
        (verdict.sunk, "sunk"),
        (verdict.on_road, "on_road"),
        (verdict.low_crown, "low_crown"),
    ];
    named.iter().filter(|(on, _)| *on).map(|(_, name)| *name).collect::<Vec<_>>().join("+")
}

/// Where a copy is from: the pass it is in, the kind of solid, the tile, the solid and the family it counts under.
struct Tag<'a> {
    phase: &'a str,
    local: bool,
    section: &'a str,
    solid: &'a str,
    family: &'a str,
}

fn note(pass: &mut Pass, tag: &Tag, at: &Placement, verdict: &Verdict) {
    match tag.local {
        true => pass.local.add(verdict),
        false => pass.world.add(verdict),
    }
    pass.by_family.entry(tag.family.to_string()).or_default().add(verdict);
    if !verdict.any() {
        return;
    }
    let ground = verdict.ground.map_or("-".to_string(), |g| format!("{g:.1}"));
    let kind = match tag.local {
        true => "local",
        false => "world",
    };
    pass.rows.push(format!(
        "{}\t{kind}\t{}\t{}\t{:.1}\t{:.1}\t{:.1}\t{ground}\t{}",
        tag.phase,
        tag.section,
        tag.solid,
        at.offset[0],
        at.offset[1],
        at.offset[2],
        flags(verdict)
    ));
}

#[test]
#[ignore = "needs the game (NFSMW_GAME_DIR), the pack (NFSMW_TREES_DIR) and NFSMW_TREES_REPORT"]
fn audit_the_trees_of_the_installed_world() {
    let dir = GameDir::open(env_path("NFSMW_GAME_DIR")).unwrap();
    let pack_dir = env_path("NFSMW_TREES_DIR");
    let profile = parse_model_swap_profile(&std::fs::read_to_string(pack_dir.join("profile.toml")).unwrap()).unwrap();
    let models = blackbox_solid::read_solids(&std::fs::read(pack_dir.join("GEOMETRY.BIN")).unwrap()).unwrap();
    let by_name: HashMap<&str, &blackbox_solid::Solid> = models.iter().map(|m| (m.name.as_str(), m)).collect();
    let mut swaps: HashMap<u32, Swapped> = HashMap::new();
    for swap in &profile.swaps {
        let Some(model) = by_name.get(swap.model.as_str()) else { continue };
        let Ok(model) = Prepared::new(model) else { continue };
        swaps.insert(bstring_hash(&swap.solid), Swapped { model, fit: swap.fit });
    }
    let index = WorldIndex::open(&dir, DEFAULT_TRACK).unwrap();
    let roads = index.road_network.as_ref().map(Roads::from_network);
    let mut shared = HashMap::new();
    for i in index.shared() {
        let section = &index.sections[i];
        let bytes = dir.read_range(&index.stream_file, u64::from(section.file_offset), section.size as usize).unwrap();
        for solid in parse_section(i, &bytes).unwrap().solids {
            shared.insert(solid.name_hash, solid);
        }
    }
    let off = FitLimits { settle: false, ..profile.limits };
    let on = FitLimits { settle: true, ..profile.limits };
    let (mut before, mut after) = (Pass::default(), Pass::default());
    let mut no_terrain = 0usize;
    for (i, section) in index.sections.iter().enumerate() {
        let bytes = dir.read_range(&index.stream_file, u64::from(section.file_offset), section.size as usize).unwrap();
        let data = parse_section(i, &bytes).unwrap();
        let terrain = Terrain::of_solids(&data.solids);
        no_terrain += usize::from(terrain.is_none() && section.is_spatial());
        let near = Neighbours::of_section(&data, |hash| swaps.contains_key(&hash));
        let site = Site { terrain: terrain.as_ref(), roads: roads.as_ref(), near: &near };
        for solid in &data.solids {
            let Some(base) = world_copy_base(&solid.name) else { continue };
            let Some(swapped) = swaps.get(&bstring_hash(base)) else { continue };
            let frame = Frame::World { mirrored: false };
            for (limits, pass, phase) in [(&off, &mut before, "before"), (&on, &mut after, "after")] {
                let Ok((placements, _, settled)) =
                    plan_copies(&swapped.model, solid, swapped.fit, frame, limits, &site)
                else {
                    continue;
                };
                pass.settled.merge(settled);
                pass.dropped += settled.dropped;
                for at in &placements {
                    let verdict = judge(swapped.model.measure(), at, &site);
                    let tag = Tag { phase, local: false, section: &section.name, solid: &solid.name, family: base };
                    note(pass, &tag, at, &verdict);
                }
            }
        }
        // The single trees the scenery places with a matrix: each stands at its instance (the foot of a local copy is the solid's origin).
        for scenery in &data.scenery {
            for instance in &scenery.instances {
                let Some(info) = scenery.info_of(instance) else { continue };
                let Some(key) = info.best_solid() else { continue };
                let (Some(swapped), Some(stock)) = (swaps.get(&key), shared.get(&key)) else { continue };
                if matches!(swapped.fit, SwapFit::Row) {
                    continue;
                }
                let local = Frame::Local;
                for (limits, pass, phase) in [(&off, &mut before, "before"), (&on, &mut after, "after")] {
                    let Ok((placements, _, _)) = plan_copies(&swapped.model, stock, swapped.fit, local, limits, &site)
                    else {
                        continue;
                    };
                    for at in &placements {
                        let at = Placement { offset: instance.position, ..*at };
                        let verdict = judge(swapped.model.measure(), &at, &site);
                        let tag =
                            Tag { phase, local: true, section: &section.name, solid: &stock.name, family: &stock.name };
                        note(pass, &tag, &at, &verdict);
                    }
                }
            }
        }
    }
    let mut summary = String::new();
    let _ = writeln!(summary, "tiles without terrain solids: {no_terrain}");
    for (name, pass) in [("before (the way the set was)", &before), ("after (settled)", &after)] {
        let _ = writeln!(summary, "{name}\n  world-space copies: {}", pass.world.line());
        let _ = writeln!(summary, "  single trees at their instances: {}", pass.local.line());
        let _ = writeln!(summary, "  settling: {:?}", pass.settled);
    }
    let _ = writeln!(summary, "\nby family (before | after)");
    let empty = Counts::default();
    for (family, counts) in &before.by_family {
        let later = after.by_family.get(family).unwrap_or(&empty);
        let _ = writeln!(summary, "{family}\t{}\t|\t{}", counts.line(), later.line());
    }
    let report = env_path("NFSMW_TREES_REPORT");
    std::fs::write(report.with_extension("audit.txt"), summary).unwrap();
    let mut rows = String::from("phase\tkind\tsection\tsolid\tx\ty\tfoot_z\tground_z\tflags\n");
    for row in before.rows.iter().chain(&after.rows) {
        rows.push_str(row);
        rows.push('\n');
    }
    std::fs::write(report.with_extension("audit.tsv"), rows).unwrap();
}
