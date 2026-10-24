//! The Custom HUD's fonts, read from the operating system: a condensed face for numbers and a neutral face for labels.
//! Nothing is bundled or redistributed. The files are read where the system keeps them, once, the first time a Custom
//! string is drawn or measured. When no candidate of a role loads, the caller draws the bundled egui font instead
//! (`ui/raster_text.rs`). Spec: `docs/specs/custom-hud-fonts.md`.
//!
//! - `windows`: the files tried on Windows (Bahnschrift with its axes set per role, then Arial Narrow and Segoe UI).
//! - `linux`: the bounded search of the system and user font folders for known file names.

#[cfg(any(target_os = "linux", test))]
mod linux;
#[cfg(any(target_os = "windows", test))]
mod windows;

#[cfg(test)]
mod tests;

use std::path::PathBuf;
use std::sync::OnceLock;

use ab_glyph::{FontArc, FontVec, VariableFont};

/// What a Custom string shows, which picks the face it is drawn in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Role {
    /// Speed, gear, position and times: a condensed, heavier face.
    Numeric,
    /// Secondary labels and notifications: a neutral face.
    Label,
    /// Spaced captions: the label face at its full width, a little lighter.
    LabelWide,
}

impl Role {
    /// Every role, in the order of the per-role arrays of this module.
    pub const ALL: [Role; 3] = [Role::Numeric, Role::Label, Role::LabelWide];

    fn index(self) -> usize {
        match self {
            Role::Numeric => 0,
            Role::Label => 1,
            Role::LabelWide => 2,
        }
    }
}

/// A variation axis of a variable font and the value it is set to.
pub type Axis = ([u8; 4], f32);

/// A font file to try: where it is, which face of a collection (0 for a single font), and the variation axes to set on
/// a variable font. Axes a font does not have are ignored.
#[derive(Clone, Debug, PartialEq)]
pub struct Candidate {
    pub path: PathBuf,
    pub index: u32,
    pub axes: &'static [Axis],
}

impl Candidate {
    pub fn new(path: impl Into<PathBuf>, index: u32, axes: &'static [Axis]) -> Self {
        Self { path: path.into(), index, axes }
    }
}

/// The fonts of the roles, in `Role::ALL` order, once. `None` for a role no candidate served.
static FONTS: OnceLock<[Option<FontArc>; 3]> = OnceLock::new();

/// The system font of `role`, or `None` when none of its candidates loads (the caller draws its fallback).
pub fn font(role: Role) -> Option<&'static FontArc> {
    FONTS.get_or_init(load_all)[role.index()].as_ref()
}

/// Loads every role from this platform's candidates and logs, once per role, which file served it.
fn load_all() -> [Option<FontArc>; 3] {
    let candidates = platform();
    Role::ALL.map(|role| {
        let tried = &candidates[role.index()];
        let found = find(tried);
        report(role, tried, found.as_ref().map(|(candidate, _)| candidate));
        found.map(|(_, font)| FontArc::from(font))
    })
}

/// The first candidate that reads and parses, with its axes set. A missing or unreadable file, or one that is not a
/// font, falls through to the next.
pub fn find(candidates: &[Candidate]) -> Option<(Candidate, FontVec)> {
    candidates.iter().find_map(|candidate| load(candidate).map(|font| (candidate.clone(), font)))
}

fn load(candidate: &Candidate) -> Option<FontVec> {
    let bytes = std::fs::read(&candidate.path).ok()?;
    let mut font = FontVec::try_from_vec_and_index(bytes, candidate.index).ok()?;
    // A variation is per font instance, so each role keeps a font of its own.
    for (axis, value) in candidate.axes {
        font.set_variation(axis, *value);
    }
    Some(font)
}

fn report(role: Role, tried: &[Candidate], found: Option<&Candidate>) {
    if let Some(candidate) = found {
        log::info!("Custom HUD {role:?} font: {} (face {})", candidate.path.display(), candidate.index);
        return;
    }
    let paths: Vec<String> = tried.iter().map(|candidate| candidate.path.display().to_string()).collect();
    log::info!(
        "Custom HUD {role:?} font: no system font loaded (tried {paths:?}); the bundled font stands in, narrowed"
    );
}

/// The candidates of each role on Windows: the system font folder, `%WINDIR%\Fonts`.
#[cfg(target_os = "windows")]
fn platform() -> [Vec<Candidate>; 3] {
    let windir = std::env::var_os("WINDIR").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(r"C:\Windows"));
    windows::candidates(&windir.join("Fonts"))
}

/// The candidates of each role on Linux: the system fonts and the user's own, searched by file name.
#[cfg(target_os = "linux")]
fn platform() -> [Vec<Candidate>; 3] {
    let mut roots = vec![PathBuf::from("/usr/share/fonts")];
    if let Some(home) = std::env::var_os("HOME") {
        roots.push(PathBuf::from(home).join(".local/share/fonts"));
    }
    linux::candidates(&roots)
}

/// The candidates of each role on macOS: DIN Condensed for numbers, then Avenir Next Condensed, then Helvetica.
#[cfg(target_os = "macos")]
fn platform() -> [Vec<Candidate>; 3] {
    let din = Candidate::new("/System/Library/Fonts/Supplemental/DIN Condensed Bold.ttf", 0, &[]);
    let avenir = Candidate::new("/System/Library/Fonts/Avenir Next Condensed.ttc", 0, &[]);
    let helvetica = Candidate::new("/System/Library/Fonts/Helvetica.ttc", 0, &[]);
    [vec![din, avenir.clone(), helvetica.clone()], vec![avenir, helvetica.clone()], vec![helvetica]]
}

/// No known font folders: every role falls back to the bundled font.
#[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
fn platform() -> [Vec<Candidate>; 3] {
    Default::default()
}
