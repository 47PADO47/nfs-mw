//! One entity's snapshot buffer and its drawn pose.

use std::collections::VecDeque;

use nfsmw_sim::{EntityKind, EntityView, Tick};

use super::curve::{extrapolate, interpolate, is_teleport, span_seconds};
use super::{DrawMode, Drawn, InterpConfig, TICK_MS};
use crate::geom::{self, IDENTITY, Quat, Vec3};

/// The offset that hides the jump from an extrapolated pose back to the interpolated one.
#[derive(Debug, Clone, Copy)]
struct Blend {
    position: Vec3,
    rotation: Quat,
    remaining_ms: f64,
    /// Made by this sample: shown whole before it starts to shrink.
    fresh: bool,
}

/// One entity: its records by tick, its first tick and its removal tick.
#[derive(Debug, Clone)]
pub(super) struct Track {
    /// Records in ascending tick order.
    records: VecDeque<(Tick, EntityView)>,
    first_tick: Tick,
    removed_at: Option<Tick>,
    blend: Option<Blend>,
    /// The display tick of the last sample, and the record extrapolated from then (if it was extrapolating).
    last_display: Option<f64>,
    extrapolated_from: Option<(Tick, EntityView)>,
}

impl Track {
    pub fn new(tick: Tick, view: EntityView) -> Self {
        let mut records = VecDeque::new();
        records.push_back((tick, view));
        Self { records, first_tick: tick, removed_at: None, blend: None, last_display: None, extrapolated_from: None }
    }

    /// Store the record of `tick`, replacing one of the same tick and keeping at most `cap` records (the newest).
    /// A record older than every kept one in a full buffer is dropped.
    pub fn push(&mut self, tick: Tick, view: EntityView, cap: usize) {
        let index = self.records.partition_point(|(t, _)| *t < tick);
        if let Some(slot) = self.records.get_mut(index).filter(|(t, _)| *t == tick) {
            slot.1 = view;
            return;
        }
        if index == 0 && self.records.len() >= cap {
            return;
        }
        self.records.insert(index, (tick, view));
        self.first_tick = self.first_tick.min(tick);
        while self.records.len() > cap.max(2) {
            self.records.pop_front();
        }
    }

    /// The entity goes at display tick `tick`.
    pub fn remove_at(&mut self, tick: Tick) {
        self.removed_at = Some(self.removed_at.map_or(tick, |t| t.min(tick)));
    }

    pub fn newest_tick(&self) -> Option<Tick> {
        self.records.back().map(|(t, _)| *t)
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// Whether the entity is gone at display tick `r`: removed, or silent for longer than the forget time.
    pub fn is_gone(&self, r: f64, config: &InterpConfig) -> bool {
        if self.removed_at.is_some_and(|t| r >= f64::from(t)) {
            return true;
        }
        let newest = self.newest_tick().map_or(f64::NEG_INFINITY, f64::from);
        (r - newest) * TICK_MS > config.forget_after_ms
    }

    /// The drawn record at display tick `r`, or `None` before the entity appears or after it is removed.
    pub fn sample(&mut self, r: f64, config: &InterpConfig) -> Option<Drawn> {
        if !r.is_finite() || r < f64::from(self.first_tick) || self.removed_at.is_some_and(|t| r >= f64::from(t)) {
            return None;
        }
        let dt_ms = self.last_display.map_or(0.0, |last| ((r - last) * TICK_MS).max(0.0));
        self.last_display = Some(r);
        let after = self.records.partition_point(|(t, _)| f64::from(*t) <= r);
        let (a_tick, a) = *self.records.get(after.saturating_sub(1))?;
        let next = self.records.get(after).copied().filter(|_| after > 0);
        let (mut view, mode, late_ms) = match next {
            Some((b_tick, b)) => self.between(r, (a_tick, &a), (b_tick, &b), config),
            None => self.beyond(r, a_tick, &a, config),
        };
        self.apply_blend(&mut view, dt_ms, config);
        let fade = self.fade(view.kind, r, mode, late_ms, config);
        Some(Drawn { view, mode, fade, late_ms })
    }

    /// Between two records, starting the blend back if the last sample was extrapolating.
    fn between(
        &mut self,
        r: f64,
        (a_tick, a): (Tick, &EntityView),
        (b_tick, b): (Tick, &EntityView),
        config: &InterpConfig,
    ) -> (EntityView, DrawMode, f64) {
        let span = span_seconds(a_tick, b_tick);
        let u = ((r - f64::from(a_tick)) / f64::from(b_tick.wrapping_sub(a_tick)).max(1.0)).clamp(0.0, 1.0) as f32;
        if is_teleport(a, b, span, config) {
            // No smoothing across a teleport: hold the first record until the second's tick.
            self.blend = None;
            self.extrapolated_from = None;
            return (*a, DrawMode::Interpolated, 0.0);
        }
        let view = interpolate(a, b, u, span);
        if let Some((tick, from)) = self.extrapolated_from.take() {
            let was = extrapolate(&from, (r - f64::from(tick)) * TICK_MS, config);
            self.blend = Some(Blend {
                position: geom::sub(was.position, view.position),
                rotation: geom::mul(was.orientation, geom::conjugate(view.orientation)),
                remaining_ms: config.blend_ms,
                fresh: true,
            });
        }
        (view, DrawMode::Interpolated, 0.0)
    }

    /// At or past the newest record.
    fn beyond(
        &mut self,
        r: f64,
        tick: Tick,
        newest: &EntityView,
        config: &InterpConfig,
    ) -> (EntityView, DrawMode, f64) {
        let late_ms = ((r - f64::from(tick)) * TICK_MS).max(0.0);
        if late_ms <= 0.0 {
            self.extrapolated_from = None;
            return (*newest, DrawMode::Interpolated, 0.0);
        }
        self.extrapolated_from = Some((tick, *newest));
        let mode = match late_ms >= config.extrapolate_limit_ms {
            true => DrawMode::Lagging,
            false => DrawMode::Extrapolated,
        };
        (extrapolate(newest, late_ms, config), mode, late_ms)
    }

    /// Add what is left of the blend offset, which shrinks linearly to nothing over the blend time.
    fn apply_blend(&mut self, view: &mut EntityView, dt_ms: f64, config: &InterpConfig) {
        let Some(blend) = self.blend.as_mut() else {
            return;
        };
        if !blend.fresh {
            blend.remaining_ms -= dt_ms;
        }
        blend.fresh = false;
        if blend.remaining_ms <= 0.0 || config.blend_ms <= 0.0 {
            self.blend = None;
            return;
        }
        let share = (blend.remaining_ms / config.blend_ms) as f32;
        view.position = geom::add(view.position, geom::scale(blend.position, share));
        let turn = geom::slerp(IDENTITY, blend.rotation, share);
        view.orientation = geom::mul(turn, view.orientation);
    }

    fn fade(&self, kind: EntityKind, r: f64, mode: DrawMode, late_ms: f64, config: &InterpConfig) -> f32 {
        let ai = matches!(kind, EntityKind::TrafficCar | EntityKind::PoliceCar);
        if ai && mode == DrawMode::Lagging && late_ms >= config.extrapolate_limit_ms + config.fade_out_after_ms {
            return 0.0;
        }
        if kind != EntityKind::TrafficCar || config.fade_in_ms <= 0.0 {
            return 1.0;
        }
        (((r - f64::from(self.first_tick)) * TICK_MS) / config.fade_in_ms).clamp(0.0, 1.0) as f32
    }
}
