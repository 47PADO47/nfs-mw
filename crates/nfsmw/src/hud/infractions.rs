//! The infraction notifier: four rows under the generic infraction icon, and the chase's infraction count beside it
//! (`docs/specs/hud-pursuit.md`, "Infraction notifier"). An infraction new to the chase takes the first row resting in
//! its `HIDE` script: the row plays its appear (it stays up 2.4 s, then leaves and hides by its own chain) with the
//! infraction's language label in both of its strings, the text and its shadow. Each new infraction also restarts the
//! icon's appear, whose messages fade in the count, the `INFRACTIONS` header and the icon's rings, and sets the count.
//! Once no row is out the icon plays its leave, whose messages take the rest away.

use blackbox_feng::package::ObjectKind;
use blackbox_feng::{ObjectRef, PackageId, Runtime, fe_hash_upper};
use blackbox_pursuit::{Infraction, InfractionLog};

/// The icon, the count, the header group and the rows (`INFRACTION_GROUP_1` to `_4`), as the package names them.
pub(super) const ICON: &str = "GENERIC_ICONS";
pub(super) const COUNT: &str = "TOTAL_INFRACTIONS_TEXT";
pub(super) const HEADER: &str = "INFRACTIONS_HEADER_GROUP";
const ROWS: usize = 4;
/// The scripts the notifier plays and tests: a row's and the icon's appear and leave, the rest state, and the icon's
/// pulse that its appear chains to.
pub(super) const APPEAR: u32 = 0x5079_C8F8;
const LEAVE: u32 = 0x0331_13AC;
pub(super) const HIDE: u32 = 0x0016_A259;
const PULSE: u32 = 0x0382_6A28;

/// The language label a row shows for `infraction` **[exe]**. The labels are named one way and read another:
/// `INFRACTION_SPEEDING` reads "Excessive Speeding" and `INFRACTION_RACING` reads "Speeding".
pub fn label(infraction: Infraction) -> u32 {
    match infraction {
        Infraction::Speeding => 0x1F87_313A,
        Infraction::Racing => 0x6DF0_8AFF,
        Infraction::RecklessDriving => 0x2185_7819,
        Infraction::Assault => 0x6643_2B02,
        Infraction::HitAndRun => 0x3E1B_41F3,
        Infraction::DamageToProperty => 0x98AC_2BAE,
        Infraction::ResistingArrest => 0xC03A_A693,
        Infraction::OffRoad => 0x50F8_3C6C,
    }
}

/// The chase's infractions the rows have told so far.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct Told {
    log: InfractionLog,
    /// A chase was fed last update.
    chase: bool,
}

impl Told {
    /// The infractions of this update's chase log (`None` with no chase) that no row has told yet, in HUD order. A chase
    /// that starts, or a log that lost an infraction it had (the next chase's), starts again from none told.
    pub fn take(&mut self, now: Option<InfractionLog>) -> Vec<Infraction> {
        let Some(now) = now else {
            self.chase = false;
            return Vec::new();
        };
        if !self.chase || self.log.iter().any(|infraction| !now.contains(infraction)) {
            self.log = InfractionLog::default();
        }
        self.chase = true;
        let new = now.iter().filter(|infraction| !self.log.contains(*infraction)).collect();
        self.log = now;
        new
    }
}

/// The notifier's objects in the package.
pub struct Notifier {
    icon: Option<ObjectRef>,
    count: Option<ObjectRef>,
    /// Each row's group and the strings under it.
    rows: Vec<(ObjectRef, Vec<ObjectRef>)>,
    told: Told,
}

impl Notifier {
    pub fn new(rt: &Runtime, package: PackageId) -> Self {
        let rows = (1..=ROWS)
            .filter_map(|n| rt.find(package, fe_hash_upper(&format!("INFRACTION_GROUP_{n}"))))
            .map(|row| (row, strings_under(rt, row)))
            .collect();
        Self {
            icon: rt.find(package, fe_hash_upper(ICON)),
            count: rt.find(package, fe_hash_upper(COUNT)),
            rows,
            told: Told::default(),
        }
    }

    /// One HUD update: tells this update's new infractions (`log` is the chase's, `None` with no chase), then lets the
    /// icon leave once no row is out. Call it before `Runtime::update`.
    pub fn apply(&mut self, rt: &mut Runtime, log: Option<InfractionLog>) {
        let count = log.map_or(0, |log| log.count());
        for infraction in self.told.take(log) {
            self.tell(rt, infraction, count);
        }
        self.settle(rt);
    }

    /// A new infraction: the first resting row shows it (none does while all four are out), the icon appears again and
    /// the count reads the chase's infractions.
    fn tell(&self, rt: &mut Runtime, infraction: Infraction, count: u32) {
        if let Some((row, strings)) = self.rows.iter().find(|(row, _)| rt.script_of(*row) == Some(HIDE)) {
            rt.run_script(*row, APPEAR);
            for string in strings {
                rt.set_label(*string, label(infraction));
            }
        }
        if let Some(icon) = self.icon {
            rt.run_script(icon, APPEAR);
        }
        if let Some(text) = self.count {
            rt.set_text(text, count.to_string());
        }
    }

    /// With every row resting, an icon that is up (appearing or pulsing) leaves.
    fn settle(&self, rt: &mut Runtime) {
        let Some(icon) = self.icon else { return };
        if self.rows.iter().any(|(row, _)| rt.script_of(*row) != Some(HIDE)) {
            return;
        }
        if matches!(rt.script_of(icon), Some(APPEAR | PULSE)) {
            rt.run_script(icon, LEAVE);
        }
    }
}

/// The string objects directly under `group`.
fn strings_under(rt: &Runtime, group: ObjectRef) -> Vec<ObjectRef> {
    let Some(package) = rt.package(group.package) else { return Vec::new() };
    let Some(guid) = package.objects.get(group.index).map(|o| o.guid) else { return Vec::new() };
    package
        .objects
        .iter()
        .enumerate()
        .filter(|(_, o)| o.parent == Some(guid) && o.kind == ObjectKind::String)
        .map(|(index, _)| ObjectRef { package: group.package, index })
        .collect()
}

#[cfg(test)]
mod tests {
    use blackbox_pursuit::Fines;

    use super::*;

    const FINES: Fines = Fines::new([150, 350, 1000, 350, 300, 100, 300, 75]);

    fn log(infractions: &[Infraction]) -> InfractionLog {
        let mut log = InfractionLog::default();
        for infraction in infractions {
            log.record(*infraction, &FINES);
        }
        log
    }

    #[test]
    fn each_infraction_is_told_once_in_a_chase() {
        let mut told = Told::default();
        assert!(told.take(Some(log(&[]))).is_empty(), "a clean chase tells nothing");
        assert_eq!(told.take(Some(log(&[Infraction::Speeding]))), [Infraction::Speeding]);
        assert!(told.take(Some(log(&[Infraction::Speeding]))).is_empty(), "the same log again tells nothing");
        let both = log(&[Infraction::Speeding, Infraction::OffRoad, Infraction::Assault]);
        assert_eq!(told.take(Some(both)), [Infraction::Assault, Infraction::OffRoad], "the new ones in HUD order");
    }

    #[test]
    fn a_new_chase_tells_its_infractions_again() {
        let mut told = Told::default();
        told.take(Some(log(&[Infraction::Speeding])));
        assert!(told.take(None).is_empty(), "no chase, nothing to tell");
        assert_eq!(told.take(Some(log(&[Infraction::Speeding]))), [Infraction::Speeding], "a chase that starts");
        told.take(Some(log(&[Infraction::Speeding, Infraction::Racing])));
        assert_eq!(
            told.take(Some(log(&[Infraction::Racing]))),
            [Infraction::Racing],
            "a log that lost one is the next chase's"
        );
    }

    #[test]
    fn every_infraction_has_its_own_label() {
        let mut labels: Vec<u32> = Infraction::ALL.into_iter().map(label).collect();
        labels.sort_unstable();
        labels.dedup();
        assert_eq!(labels.len(), Infraction::ALL.len());
        assert_eq!(label(Infraction::Assault), 0x6643_2B02, "INFRACTION_ASSULTING_OFFICER: Ramming a Police Vehicle");
    }
}
