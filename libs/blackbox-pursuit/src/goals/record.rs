//! The slot records of the goals and the kinds they belong to. Spec: `docs/specs/ai-pursuit-goals.md` sections 1 and 2.

use glam::Vec2;

/// The goal kinds that have in-position records or a ram action here. The follow, herd and stagger goals have no
/// in-position record in this module (their slots are in `formation::slots`). The pull-over is the ram action with the
/// name test of section 7; its factory kind number is not identified (open question Q1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GoalKind {
    Pit,
    BoxIn,
    RollingBlock,
    PullOver,
}

/// One slot record of a goal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Record {
    /// The spec's label for the slot.
    pub name: &'static str,
    /// Pursuit slot A, in the target's frame (`x` to its right, `y` forward, metres): where the cop waits while it
    /// chases in formation.
    pub pursuit: Vec2,
    /// In-position offset B, same frame, metres: where the cop drives once the finisher is armed. The record field that
    /// feeds the ram action is read through a helper whose decompile is partly garbled. The reading that B is the field
    /// the action steers to is [unconfirmed] (goals spec Q2); this module follows it.
    pub in_position: Vec2,
    /// Priority: a lower number is matched to a cop first.
    pub priority: u32,
}

/// The depth of the front in-position point of the box-in and the centre of the rolling block: `foff = 2 - 5 t`, in
/// metres. Negative is behind the target's centre. `t` is the tightness (0 loose, 1 tight).
pub fn front_depth(tightness: f32) -> f32 {
    2.0 - 5.0 * tightness
}
