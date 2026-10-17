//! Ordering: dependencies, load-order hints, cycles, missing and unsatisfied dependencies.

use std::path::PathBuf;

use super::*;
use crate::host::discovery::{Candidate, Discovery};
use crate::host::manifest::{self, Manifest};
use crate::host::reject::RejectReason;

/// A candidate built from manifest text, with a folder named after its id.
fn cand(text: &str) -> Candidate {
    let manifest: Manifest = manifest::parse(text).expect("test manifest");
    let dir = PathBuf::from(format!("mods/{}", manifest.id));
    Candidate { manifest, dir, library: None }
}

/// A manifest with the given id, version and extra sections.
fn mod_text(id: &str, version: &str, extra: &str) -> String {
    format!(
        "[mod]\nid = \"{id}\"\nname = \"{id}\"\nversion = \"{version}\"\n[api]\nblackbox_modding = \"0.1\"\n\
         [games]\nsupported = [\"*\"]\n{extra}\n"
    )
}

fn run(candidates: Vec<Candidate>) -> Resolution {
    resolve(Discovery { candidates, rejected: Vec::new() })
}

fn order_of(resolution: &Resolution) -> Vec<&str> {
    resolution.ordered.iter().map(|c| c.manifest.id.as_str()).collect()
}

fn reason_of(resolution: &Resolution, id: &str) -> Option<RejectReason> {
    resolution.rejected.iter().find(|r| r.id.as_deref() == Some(id)).map(|r| r.reason.clone())
}

#[test]
fn independent_mods_come_out_in_id_order() {
    let resolution = run(vec![
        cand(&mod_text("c.mod", "1.0.0", "")),
        cand(&mod_text("a.mod", "1.0.0", "")),
        cand(&mod_text("b.mod", "1.0.0", "")),
    ]);
    assert_eq!(order_of(&resolution), ["a.mod", "b.mod", "c.mod"]);
    assert!(resolution.rejected.is_empty());
}

#[test]
fn a_dependency_loads_before_its_dependant_whatever_the_ids() {
    let resolution = run(vec![
        cand(&mod_text("a.app", "1.0.0", "[dependencies]\n\"z.base\" = \">=1.0\"\n")),
        cand(&mod_text("z.base", "1.2.0", "")),
    ]);
    assert_eq!(order_of(&resolution), ["z.base", "a.app"]);
}

#[test]
fn load_after_orders_without_a_hard_dependency() {
    let resolution = run(vec![
        cand(&mod_text("a.first", "1.0.0", "[load]\nafter = [\"z.other\"]\n")),
        cand(&mod_text("z.other", "1.0.0", "")),
    ]);
    assert_eq!(order_of(&resolution), ["z.other", "a.first"]);
}

#[test]
fn load_before_orders_the_other_way() {
    let resolution = run(vec![
        cand(&mod_text("z.early", "1.0.0", "[load]\nbefore = [\"a.late\"]\n")),
        cand(&mod_text("a.late", "1.0.0", "")),
    ]);
    assert_eq!(order_of(&resolution), ["z.early", "a.late"]);
}

#[test]
fn an_absent_load_hint_is_ignored() {
    let resolution = run(vec![cand(&mod_text("a.mod", "1.0.0", "[load]\nafter = [\"nobody.here\"]\n"))]);
    assert_eq!(order_of(&resolution), ["a.mod"]);
    assert!(resolution.rejected.is_empty());
}

#[test]
fn a_missing_dependency_rejects_the_dependant_and_its_dependants() {
    let resolution = run(vec![
        cand(&mod_text("a.app", "1.0.0", "[dependencies]\n\"nobody.base\" = \"*\"\n")),
        cand(&mod_text("b.plugin", "1.0.0", "[dependencies]\n\"a.app\" = \"*\"\n")),
        cand(&mod_text("c.alone", "1.0.0", "")),
    ]);
    assert_eq!(order_of(&resolution), ["c.alone"]);
    assert_eq!(reason_of(&resolution, "a.app"), Some(RejectReason::DependencyMissing { id: "nobody.base".to_owned() }));
    assert_eq!(reason_of(&resolution, "b.plugin"), Some(RejectReason::DependencyRejected { id: "a.app".to_owned() }));
}

#[test]
fn a_dependency_in_the_wrong_version_rejects_the_dependant() {
    let resolution = run(vec![
        cand(&mod_text("a.app", "1.0.0", "[dependencies]\n\"b.base\" = \">=2\"\n")),
        cand(&mod_text("b.base", "1.9.0", "")),
    ]);
    assert_eq!(order_of(&resolution), ["b.base"]);
    assert_eq!(
        reason_of(&resolution, "a.app"),
        Some(RejectReason::DependencyVersion {
            id: "b.base".to_owned(),
            requirement: ">=2".to_owned(),
            found: "1.9.0".to_owned(),
        })
    );
}

#[test]
fn a_dependency_that_discovery_rejected_rejects_the_dependant() {
    let mut discovery = Discovery::default();
    discovery.rejected.push(Rejected {
        id: Some("b.base".to_owned()),
        folder: "b".to_owned(),
        dir: PathBuf::from("mods/b"),
        reason: RejectReason::NotAllowed,
    });
    discovery.candidates.push(cand(&mod_text("a.app", "1.0.0", "[dependencies]\n\"b.base\" = \"*\"\n")));
    let resolution = resolve(discovery);
    assert!(resolution.ordered.is_empty());
    assert_eq!(reason_of(&resolution, "a.app"), Some(RejectReason::DependencyRejected { id: "b.base".to_owned() }));
}

#[test]
fn a_cycle_rejects_every_member_with_the_cycle_as_reason() {
    let resolution = run(vec![
        cand(&mod_text("a.one", "1.0.0", "[dependencies]\n\"b.two\" = \"*\"\n")),
        cand(&mod_text("b.two", "1.0.0", "[dependencies]\n\"c.three\" = \"*\"\n")),
        cand(&mod_text("c.three", "1.0.0", "[dependencies]\n\"a.one\" = \"*\"\n")),
        cand(&mod_text("d.free", "1.0.0", "")),
    ]);
    assert_eq!(order_of(&resolution), ["d.free"]);
    let members = vec!["a.one".to_owned(), "b.two".to_owned(), "c.three".to_owned()];
    assert_eq!(reason_of(&resolution, "a.one"), Some(RejectReason::Cycle { members: members.clone() }));
    assert_eq!(reason_of(&resolution, "c.three"), Some(RejectReason::Cycle { members }));
}

#[test]
fn a_cycle_through_load_hints_is_a_cycle() {
    let resolution = run(vec![
        cand(&mod_text("a.one", "1.0.0", "[load]\nafter = [\"b.two\"]\n")),
        cand(&mod_text("b.two", "1.0.0", "[load]\nafter = [\"a.one\"]\n")),
    ]);
    assert!(resolution.ordered.is_empty());
    assert!(matches!(reason_of(&resolution, "a.one"), Some(RejectReason::Cycle { .. })));
}

#[test]
fn a_mod_that_only_depends_on_a_cycle_is_rejected_but_an_unrelated_hint_is_not() {
    let resolution = run(vec![
        cand(&mod_text("a.one", "1.0.0", "[dependencies]\n\"b.two\" = \"*\"\n")),
        cand(&mod_text("b.two", "1.0.0", "[dependencies]\n\"a.one\" = \"*\"\n")),
        cand(&mod_text("c.user", "1.0.0", "[dependencies]\n\"a.one\" = \"*\"\n")),
        cand(&mod_text("d.hint", "1.0.0", "[load]\nafter = [\"a.one\"]\n")),
    ]);
    assert_eq!(order_of(&resolution), ["d.hint"]);
    assert_eq!(reason_of(&resolution, "c.user"), Some(RejectReason::DependencyRejected { id: "a.one".to_owned() }));
}

#[test]
fn the_same_inputs_always_give_the_same_order() {
    let build = || {
        run(vec![
            cand(&mod_text("m.x", "1.0.0", "")),
            cand(&mod_text("b.y", "1.0.0", "[dependencies]\n\"m.x\" = \"*\"\n")),
            cand(&mod_text("a.z", "1.0.0", "")),
        ])
    };
    assert_eq!(order_of(&build()), order_of(&build()));
    assert_eq!(order_of(&build()), ["a.z", "m.x", "b.y"]);
}
