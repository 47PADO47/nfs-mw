//! `list-screens` and `dump-screen`: what the install's FEng packages hold, as text. Names exist in a package
//! only as hashes, so objects, scripts and messages print as hex; a name the game uses can be hashed with
//! `fe_hash_upper` to find it.

use std::fmt::Write;

use blackbox_feng::Package;
use blackbox_feng::package::{ObjectDef, ObjectKind, ResponseKind, ResponseParam};

use crate::ui::Catalog;

/// One line per package: name, file, objects, buttons.
pub fn list(catalog: &Catalog) -> String {
    let mut out = String::new();
    for e in catalog.entries() {
        let Some(p) = catalog.find(&e.name) else { continue };
        let buttons = p.objects.iter().filter(|o| o.flags & BUTTON != 0).count();
        let _ = writeln!(out, "{:<40} {:<32} {:>4} objects {:>3} buttons", e.name, e.file, p.objects.len(), buttons);
    }
    out
}

/// The FEng flag of an object that takes pad focus.
pub const BUTTON: u32 = 1 << 28;

/// The whole package: resources, then the object tree with scripts and responses.
pub fn dump(p: &Package) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "{} ({}), version {:#x}", p.name, p.file_name, p.version);
    for (i, r) in p.resources.iter().enumerate() {
        let _ = writeln!(out, "  resource {i}: {:?} {} handle {:#010x}", r.kind, r.name, r.handle);
    }
    for r in &p.responses {
        let _ = writeln!(out, "  package response {:#010x}: {}", r.message, responses(&r.responses));
    }
    for t in &p.targets {
        let guids: Vec<String> = t.guids.iter().map(|g| format!("{g:#x}")).collect();
        let _ = writeln!(out, "  message {:#010x} goes to {}", t.message, guids.join(" "));
    }
    let roots: Vec<usize> =
        (0..p.objects.len()).filter(|&i| p.objects[i].parent.and_then(|g| p.find_by_guid(g)).is_none()).collect();
    for i in roots {
        tree(p, i, 0, &mut out);
    }
    out
}

fn tree(p: &Package, index: usize, depth: usize, out: &mut String) {
    let o = &p.objects[index];
    let pad = "  ".repeat(depth + 1);
    let _ = writeln!(out, "{pad}{}", line(p, o));
    for s in &o.scripts {
        let events: Vec<String> =
            s.events.iter().map(|e| format!("{:#x}->{:#x}@{}", e.message, e.target, e.time)).collect();
        let chain = s.chain.map_or(String::new(), |c| format!(" chain {c:#x}"));
        let _ = writeln!(
            out,
            "{pad}  script {:#010x} len {} end {}{chain} tracks {} events [{}]",
            s.id,
            s.length,
            s.end_behaviour(),
            s.tracks.len(),
            events.join(" ")
        );
    }
    for r in &o.responses {
        let _ = writeln!(out, "{pad}  on {:#010x}: {}", r.message, responses(&r.responses));
    }
    for (child, c) in p.objects.iter().enumerate() {
        if c.parent == Some(o.guid) && child != index && o.kind == ObjectKind::Group {
            tree(p, child, depth + 1, out);
        }
    }
}

fn line(p: &Package, o: &ObjectDef) -> String {
    let pos = o.data.position();
    let size = o.data.size();
    let resource = o.resource.and_then(|r| p.resources.get(r)).map_or(String::new(), |r| format!(" [{}]", r.name));
    let text = o.string.as_ref().map_or(String::new(), |s| format!(" {:?} label {:#x}", s.text, s.label));
    let button = if o.flags & BUTTON != 0 { " BUTTON" } else { "" };
    format!(
        "{:?} guid {:#x} name {:#010x} flags {:#x}{button} pos ({:.0},{:.0},{:.0}) size ({:.0},{:.0}) alpha {}{resource}{text}",
        o.kind,
        o.guid,
        o.name_hash,
        o.flags,
        pos.x,
        pos.y,
        pos.z,
        size.x,
        size.y,
        o.data.alpha()
    )
}

/// The language table lines (`hash text`) whose text contains `filter` (any case), or `0xHASH` for one label.
pub fn strings(table: &blackbox_text::StringTable, filter: &str) -> String {
    let mut out = String::new();
    let by_hash = filter.strip_prefix("0x").and_then(|h| u32::from_str_radix(h, 16).ok());
    let needle = filter.to_lowercase();
    for label in table.labels() {
        let Some(text) = table.get(label) else { continue };
        let hit = by_hash.map_or_else(|| text.to_lowercase().contains(&needle), |h| h == label);
        if hit {
            let _ = writeln!(out, "{label:#010x} {text:?}");
        }
    }
    out
}

fn responses(list: &[blackbox_feng::package::Response]) -> String {
    let parts: Vec<String> = list
        .iter()
        .map(|r| {
            let param = match &r.param {
                ResponseParam::Number(n) => format!("{n:#x}"),
                ResponseParam::Text(s) => format!("{s:?}"),
            };
            let kind = match r.kind {
                ResponseKind::SetScript => "script".to_owned(),
                ResponseKind::PostToFEng => "feng".to_owned(),
                ResponseKind::PostToGame => "game".to_owned(),
                ResponseKind::PostToSound => "sound".to_owned(),
                ResponseKind::Button(id) => format!("button{id:#x}"),
                ResponseKind::Package(id) => format!("package{id:#x}"),
                ResponseKind::IfScriptEquals => "if=".to_owned(),
                ResponseKind::IfScriptNotEquals => "if!=".to_owned(),
                ResponseKind::Else => "else".to_owned(),
                ResponseKind::EndIf => "endif".to_owned(),
                ResponseKind::Other(id) => format!("other{id:#x}"),
            };
            format!("{kind}({param}->{:#x})", r.target)
        })
        .collect();
    parts.join(", ")
}
