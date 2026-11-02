//! The pursuit board's backup row on a small package with its objects (names and parents as in `HUD_SingleRace.fng`):
//! which script the backing plays for the backup ETA, and the timer text. The cascade from the backing's scripts to the
//! label group is the package's own; the installed test checks it.

use blackbox_feng::package::{ObjectKind, Script, StringDef};
use blackbox_feng::{ObjectRef, PackageId, Runtime};

use super::backup_timer::{APPEAR, FLASH, LEAVE, URGENT_SECS, script_for};
use super::bind::HudBinding;
use super::minimap_tests::{object, package};
use super::pursuit::PursuitHud;
use super::state::HudState;

const BACKING: u32 = 0x8c20_a763;
const TIMER: u32 = 0xbee4_4775;

fn setup() -> (Runtime, PackageId, HudBinding) {
    let mut package = package();
    let mut board = object(ObjectKind::Group, 80, "PursuitBoardGroup", None, [0.0; 2]);
    board.name_hash = 0xde89_e070;
    let mut backing = object(ObjectKind::Image, 81, "", Some(80), [0.0; 2]);
    backing.name_hash = BACKING;
    let script = |id: u32, length: i32| Script { id, length, ..Default::default() };
    backing.scripts = vec![script(0x0016_a259, 0), script(APPEAR, 320), script(LEAVE, 320), script(FLASH, 400)];
    let mut row = object(ObjectKind::Group, 82, "", Some(80), [0.0; 2]);
    row.name_hash = 0x6a14_4066;
    let mut timer = object(ObjectKind::String, 83, "", Some(82), [0.0; 2]);
    timer.name_hash = TIMER;
    timer.string = Some(StringDef { text: "00:59.99".into(), ..Default::default() });
    package.objects.extend([board, backing, row, timer]);
    let mut rt = Runtime::new();
    let id = rt.load(package);
    let binding = HudBinding::new(&mut rt, id);
    (rt, id, binding)
}

fn chase(backup_eta: f32, cooldown: bool) -> HudState {
    HudState {
        pursuit: Some(PursuitHud { secs: 30.0, backup_eta, cooldown, ..Default::default() }),
        ..Default::default()
    }
}

fn backing(rt: &Runtime, id: PackageId) -> Option<u32> {
    let o: ObjectRef = rt.find(id, BACKING).expect("the backing");
    rt.script_of(o)
}

fn timer(rt: &Runtime, id: PackageId) -> String {
    rt.tree(id).nodes.iter().find(|n| n.name_hash == TIMER).and_then(|n| n.text.clone()).unwrap_or_default()
}

#[test]
fn the_backing_appears_far_out_flashes_in_the_last_ten_seconds_and_leaves_at_zero() {
    assert_eq!(script_for(0.0), LEAVE);
    assert_eq!(script_for(-0.2), LEAVE, "the ETA overshoots a little when the backup arrives");
    assert_eq!(script_for(URGENT_SECS), FLASH);
    assert_eq!(script_for(0.01), FLASH);
    assert_eq!(script_for(URGENT_SECS + 0.01), APPEAR);
    assert_eq!(script_for(120.0), APPEAR);
}

#[test]
fn the_row_follows_the_backup_and_shows_its_time() {
    let (mut rt, id, mut b) = setup();
    let rest = backing(&rt, id);
    b.apply(&mut rt, &chase(0.0, false));
    assert_eq!(backing(&rt, id), rest, "no backup called: the hidden row is not played out");
    b.apply(&mut rt, &chase(75.5, false));
    assert_eq!(backing(&rt, id), Some(APPEAR));
    assert_eq!(timer(&rt, id), "01:15.50");
    b.apply(&mut rt, &chase(8.0, false));
    assert_eq!(backing(&rt, id), Some(FLASH));
    assert_eq!(timer(&rt, id), "00:08.00");
    // The board shows the cool-down: the row is left as it was.
    b.apply(&mut rt, &chase(0.0, true));
    assert_eq!(backing(&rt, id), Some(FLASH));
    assert_eq!(timer(&rt, id), "00:08.00");
    b.apply(&mut rt, &chase(0.0, false));
    assert_eq!(backing(&rt, id), Some(LEAVE), "the backup arrived or was called off");
}
