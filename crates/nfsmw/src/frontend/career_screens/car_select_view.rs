//! What the career car select draws for a car: the header (strike pips, impound stamp, value lines, Accept label), the
//! marker counts and the heat meter, as pure data ([`header`]) and the calls that set them on the package.

use blackbox_feng::fe_hash_upper;
use nfsmw_career::economy::aftermath::{CarSelectEntry, CarSelectInfo, CarStatus};
use nfsmw_career::economy::cooling::{HeatMeter, Segment};

use super::car_lot::{set_label, set_text};
use crate::frontend::infractions::money;
use crate::frontend::logic::Cx;

// Objects of `Car_Select.fng` by name hash (its dump, `nfsmw dump-screen Car_Select.fng`; the roles are the original's
// as the specs give them).
/// The lot's price and cash lines, hidden in this mode.
const LOT_ONLY: [u32; 4] = [0x1939_8802, 0x1930_b057, 0x20d1_13dc, 0x20c8_3c31];
/// Strike pips: the group, the two move scripts' targets, the fourth and fifth pip groups.
const PIP_GROUP: u32 = 0x7572_1326;
const PIP_MOVE: [u32; 2] = [0xf9a5_ce86, 0xebf0_016e];
const PIP_X4: u32 = 0x5bc7_8037;
const PIP_X5: u32 = 0x4809_5518;
/// The impound stamp: its group, the second group the strip appears in, and the value lines.
const STAMP: u32 = 0x0bc7_b91f;
const STAMP_STRIP: u32 = 0x64f3_a49c;
const LABEL_A: u32 = 0x72e7_ea88;
const LABEL_B: u32 = 0x9d97_4df3;
const VALUE_A: u32 = 0x322b_18f9;
const VALUE_B: u32 = 0x7044_a5a4;
const ACCEPT_TEXT: u32 = 0xb941_39f4;
/// Visible for a car that is free to drive (bounty and fines).
const FREE_ONLY: u32 = 0x0e9e_d0a2;
/// The marker count groups and their two number texts each.
const STRIKE_MARKERS: u32 = 0x39dc_21f9;
const STRIKE_MARKER_COUNT: [u32; 2] = [0x5b87_5870, 0xea8a_ecd9];
const RELEASE_MARKERS: u32 = 0xe998_fe99;
const RELEASE_MARKER_COUNT: [u32; 2] = [0xcc59_b910, 0xb8f9_938a];
/// The heat meter: its group, the multiplier image and its zoom twin, and the ten segments (`HEAT_BASE_LED_1..10`).
const HEAT_GROUP: u32 = 0xf249_2598;
const HEAT_X: [u32; 2] = [0x5ffe_e1d8, 0xe637_955c];

// Scripts by name hash (`fe_hash_upper`, checked against the package).
const SCRIPT_HIDE: u32 = 0x0016_a259;
const SCRIPT_SHOW: u32 = 0x001c_a7c0;
const SCRIPT_ANIMATE: u32 = 0x5a8e_4ebe;
const SCRIPT_APPEAR: u32 = 0x5079_c8f8;
const SCRIPT_NORMAL: u32 = 0x6ebb_fb68;
const SCRIPT_ON: u32 = 0x0000_063c;
const SCRIPT_OFF: u32 = 0x0000_ccfa;
const SCRIPT_COOL: u32 = 0x0013_ff6c;

// Labels of the game's strings table.
const LABEL_BOUNTY: u32 = 0x1757_4b0e;
const LABEL_FINES: u32 = 0x915f_4d26;
const LABEL_RELEASE_COST: u32 = 0x9db4_df7d;
const LABEL_CASH: u32 = 0x073b_79e0;
const LABEL_PENDING: u32 = 0xaefe_dad9;
pub(super) const LABEL_RELEASE: u32 = 0x281d_ee8a;
pub(super) const LABEL_USE_MARKER: u32 = 0xf9c7_3cc2;
const LABEL_ACCEPT: u32 = 0x2b65_a216;

/// The script a strike pip plays.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pip {
    Hide,
    Show,
    Animate,
}

/// The header of one car: what the pips, the stamp and the value lines show. Pure, so it is tested without a package.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Header {
    /// The five pips (`IMPOUND_STATE_1..5`).
    pub pips: [Pip; 5],
    /// The pip group's move script (`POS1` to `POS3`, by the strike limit).
    pub position: u8,
    pub show_pip_four: bool,
    pub show_pip_five: bool,
    /// The stamp shows (impounded or releasable) and plays its animation.
    pub stamp: bool,
    pub stamp_animates: bool,
    /// The first value line: a label, then either a number or a label as the value.
    pub label_a: u32,
    pub value_a: Value,
    pub label_b: u32,
    pub value_b: Value,
    /// The Accept button's label.
    pub accept: u32,
}

/// A value line's value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Value {
    Money(u64),
    Label(u32),
}

/// The header for a car: `info` is its record's numbers, `entry` the animation the screen opened with, `cash` the cash
/// and `release_marker` whether an Impound Release marker is held.
pub fn header(info: &CarSelectInfo, entry: CarSelectEntry, cash: u32, release_marker: bool) -> Header {
    let impounded = info.status != CarStatus::Free;
    let mut pips = [Pip::Hide; 5];
    for (i, pip) in pips.iter_mut().enumerate() {
        let number = i as u8 + 1;
        if number > info.strike_limit || number > info.strikes {
            continue;
        }
        let newest = number == info.strikes && entry == CarSelectEntry::NewStrike;
        *pip = if newest { Pip::Animate } else { Pip::Show };
    }
    let (label_a, value_a, label_b, value_b) = match info.status {
        CarStatus::Free => (LABEL_BOUNTY, Value::Money(info.bounty.into()), LABEL_FINES, Value::Money(info.fines)),
        CarStatus::Releasable => {
            (LABEL_RELEASE_COST, Value::Money(info.release_cost.into()), LABEL_CASH, Value::Money(cash.into()))
        }
        CarStatus::Impounded => {
            (LABEL_RELEASE_COST, Value::Label(LABEL_PENDING), LABEL_CASH, Value::Money(cash.into()))
        }
    };
    let can_pay = info.status == CarStatus::Releasable && cash >= info.release_cost;
    let accept = match (can_pay, release_marker && impounded) {
        (true, _) => LABEL_RELEASE,
        (false, true) => LABEL_USE_MARKER,
        (false, false) => LABEL_ACCEPT,
    };
    Header {
        pips,
        position: match info.strike_limit {
            5 => 3,
            4 => 2,
            _ => 1,
        },
        show_pip_four: info.strike_limit >= 4,
        show_pip_five: info.strike_limit >= 5,
        stamp: impounded,
        stamp_animates: impounded && entry == CarSelectEntry::Impounded,
        label_a,
        value_a,
        label_b,
        value_b,
        accept,
    }
}

// The drawing helpers: each only sets the package's objects, so they are exercised by the installed test.

pub(super) fn hide_header(cx: &mut Cx) {
    for name in [PIP_GROUP, STAMP_STRIP, STRIKE_MARKERS, RELEASE_MARKERS, HEAT_GROUP, FREE_ONLY] {
        cx.hide(name, true);
    }
    cx.script(STAMP, SCRIPT_HIDE);
}

fn draw_pips(cx: &mut Cx, head: &Header) {
    cx.hide(PIP_GROUP, head.stamp);
    for (i, pip) in head.pips.iter().enumerate() {
        let script = match pip {
            Pip::Hide => SCRIPT_HIDE,
            Pip::Show => SCRIPT_SHOW,
            Pip::Animate => SCRIPT_ANIMATE,
        };
        cx.script(fe_hash_upper(&format!("IMPOUND_STATE_{}", i + 1)), script);
    }
    let position = fe_hash_upper(&format!("POS{}", head.position));
    for target in PIP_MOVE {
        cx.script(target, position);
    }
    cx.script(PIP_X4, if head.show_pip_four { SCRIPT_SHOW } else { SCRIPT_HIDE });
    cx.script(PIP_X5, if head.show_pip_five { SCRIPT_SHOW } else { SCRIPT_HIDE });
}

fn draw_stamp(cx: &mut Cx, head: &Header) {
    cx.hide(STAMP_STRIP, !head.stamp);
    let (stamp, strip) = match (head.stamp, head.stamp_animates) {
        (false, _) => (SCRIPT_HIDE, SCRIPT_HIDE),
        (true, true) => (SCRIPT_ANIMATE, SCRIPT_APPEAR),
        (true, false) => (SCRIPT_NORMAL, SCRIPT_APPEAR),
    };
    cx.script(STAMP, stamp);
    cx.script(STAMP_STRIP, strip);
}

fn draw_lines(cx: &mut Cx, head: &Header) {
    set_label(cx, LABEL_A, head.label_a);
    set_label(cx, LABEL_B, head.label_b);
    for (object, value) in [(VALUE_A, head.value_a), (VALUE_B, head.value_b)] {
        match value {
            Value::Money(amount) => set_text(cx, object, money_of(amount)),
            Value::Label(label) => set_label(cx, object, label),
        }
    }
}

fn draw_count(cx: &mut Cx, group: u32, texts: [u32; 2], count: u32, visible: bool) {
    cx.hide(group, !visible);
    for text in texts {
        set_text(cx, text, format!("{count:2}"));
    }
}

/// The meter: the multiplier icon (`HEAT_X<n>`) and the ten segments (`HEAT_BASE_LED_1..10`).
fn draw_heat(cx: &mut Cx, meter: &HeatMeter) {
    cx.hide(HEAT_GROUP, false);
    let texture = fe_hash_upper(&format!("HEAT_X{}", meter.multiplier));
    for name in HEAT_X {
        if let Some(o) = cx.object(name) {
            cx.rt.set_texture(o, texture);
        }
    }
    for (i, segment) in meter.segments.iter().enumerate() {
        let script = match segment {
            Segment::Off => SCRIPT_OFF,
            Segment::Cool => SCRIPT_COOL,
            Segment::On => SCRIPT_ON,
        };
        cx.script(fe_hash_upper(&format!("HEAT_BASE_LED_{}", i + 1)), script);
    }
}

/// The title of this mode's header: "Career" (the label the lot's owned tab uses too).
pub(super) const LABEL_TITLE: u32 = 0x5415_c3f1;

/// Sets the header title.
pub(super) fn set_title(cx: &mut Cx) {
    set_label(cx, super::car_lot::TAB_TITLE, LABEL_TITLE);
}

/// Hides the lot's price and cash lines, which this mode does not use.
pub(super) fn hide_lot_lines(cx: &mut Cx) {
    for name in LOT_ONLY {
        cx.hide(name, true);
    }
}

/// An amount as the ticket shows it (commas), saturating at the largest the screen's number type holds.
pub(super) fn money_of(amount: u64) -> String {
    money(u32::try_from(amount).unwrap_or(u32::MAX))
}

/// Draws the header, the marker counts and the heat meter of a car.
pub(super) fn draw_car(
    cx: &mut Cx,
    info: &CarSelectInfo,
    entry: CarSelectEntry,
    cash: u32,
    strike_markers: u32,
    release_markers: u32,
) {
    let head = header(info, entry, cash, release_markers > 0);
    draw_pips(cx, &head);
    draw_stamp(cx, &head);
    draw_lines(cx, &head);
    set_label(cx, ACCEPT_TEXT, head.accept);
    let free = info.status == CarStatus::Free;
    cx.hide(FREE_ONLY, !free);
    draw_count(cx, STRIKE_MARKERS, STRIKE_MARKER_COUNT, strike_markers, free && strike_markers > 0);
    draw_count(cx, RELEASE_MARKERS, RELEASE_MARKER_COUNT, release_markers, !free && release_markers > 0);
    draw_heat(cx, &HeatMeter::of(info.heat));
}
