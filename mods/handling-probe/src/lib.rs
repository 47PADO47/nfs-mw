//! `handling-probe`: a fixture for the `bb.handling` host seam. It registers one tyre hook and answers every wheel of the
//! player's car with a trivial model, so the host tests can load a real library, call a real callback and watch what the
//! host does with the answers.
//!
//! The model is chosen by `model = "..."` in the mod's `config.toml` (read through `bb.config`):
//!
//! - `linear` (the default): force proportional to slip with a friction limit, wheel spin integrated with the given torques.
//! - `friction-circle`: a second, differently built model: a constant friction coefficient on the combined slip. It proves
//!   the hook is not shaped around one model.
//! - `not-handled`: declines every wheel, so the built-in tyres run.
//! - `panics`: panics on every call. The SDK catches it, the host counts a fault, and the fifth quarantines the hook.
//! - `nan`: answers a force that is not a number. The host refuses it and counts a fault.
//!
//! Whatever the model, a call for a vehicle that is not the player's is declined: the hook checks the role it is told, as a
//! mod that wants to touch one kind of vehicle must. With `bb.vehicles` 1.3 the probe reads the car's `spec` whenever the
//! `vehicle_revision` of a query changes and logs a line.

mod models;

use blackbox_modding::sdk::{
    Ctx, Error, HandlingHookId, Level, Mod, ModResult, Role, TyreQuery, TyreResult, declare_mod,
};

/// What the probe's hook does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Variant {
    /// The linear tyre.
    Linear,
    /// The constant-friction tyre.
    FrictionCircle,
    /// Decline every wheel.
    NotHandled,
    /// Panic on every call.
    Panics,
    /// Answer a force that is not a number.
    Nan,
}

impl Variant {
    /// The variant a config value names.
    pub fn parse(name: &str) -> Option<Variant> {
        match name {
            "linear" => Some(Variant::Linear),
            "friction-circle" => Some(Variant::FrictionCircle),
            "not-handled" => Some(Variant::NotHandled),
            "panics" => Some(Variant::Panics),
            "nan" => Some(Variant::Nan),
            _ => None,
        }
    }
}

/// The value of `model = "..."` in a config text, if it has one.
pub fn model_name(config: &str) -> Option<&str> {
    config.lines().find_map(|line| {
        let (key, value) = line.split_once('=')?;
        (key.trim() == "model").then(|| value.trim().trim_matches('"'))
    })
}

/// The mod: the hook it registered, and the variant it runs.
pub struct Probe {
    hook: HandlingHookId,
    variant: Variant,
}

/// The variant the config asks for; the linear tyre without a config.
fn configured(ctx: &Ctx) -> ModResult<Variant> {
    let text = ctx.config().and_then(|config| config.text().ok()).unwrap_or_default();
    let Some(name) = model_name(&text) else {
        return Ok(Variant::Linear);
    };
    Variant::parse(name).ok_or(Error::Arg)
}

/// Read the car's spec when the revision changes, and log what came back.
fn note_revision(ctx: &Ctx, revision: u32, seen: &mut u32) {
    if revision == *seen {
        return;
    }
    *seen = revision;
    let Some(vehicles) = ctx.vehicles() else {
        return;
    };
    match vehicles.spec() {
        Ok(spec) => ctx.log(
            Level::Info,
            &format!(
                "handling-probe: spec revision {revision}: {:.0} kg, {} gears, {:.0} W",
                spec.mass_kg,
                spec.gear_ratios.len(),
                spec.max_power_w
            ),
        ),
        Err(error) => ctx.log(Level::Warn, &format!("handling-probe: spec not read: {error}")),
    }
}

/// The hook: one call per wheel per tick.
fn answer(variant: Variant, ctx: &mut Ctx, query: &TyreQuery, seen: &mut u32) -> Option<TyreResult> {
    if query.role != Some(Role::Player) {
        return None;
    }
    note_revision(ctx, query.vehicle_revision, seen);
    match variant {
        Variant::Linear => Some(models::linear(query)),
        Variant::FrictionCircle => Some(models::friction_circle(query)),
        Variant::NotHandled => None,
        Variant::Panics => panic!("handling-probe: the panics variant panics on purpose"),
        Variant::Nan => Some(TyreResult { longitudinal_force_n: f32::NAN, traction: 1.0, ..TyreResult::default() }),
    }
}

impl Mod for Probe {
    fn load(ctx: &mut Ctx) -> ModResult<Self> {
        let variant = configured(ctx)?;
        let handling = ctx.handling().ok_or(Error::Unsupported)?;
        let mut seen = 0_u32;
        let hook = handling.register_tyre(move |ctx, query| answer(variant, ctx, query, &mut seen))?;
        ctx.log(Level::Info, &format!("handling-probe: registered the {variant:?} tyre hook"));
        Ok(Probe { hook, variant })
    }

    fn unload(&mut self, ctx: &mut Ctx) {
        if let Some(handling) = ctx.handling() {
            let _ = handling.unregister_tyre(self.hook);
        }
        ctx.log(Level::Info, &format!("handling-probe: unloaded the {:?} tyre hook", self.variant));
    }
}

declare_mod!(Probe);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_model_name_is_read_from_a_config_text() {
        assert_eq!(model_name("model = \"panics\"\n"), Some("panics"));
        assert_eq!(model_name("# comment\nother = 1\n model=\"nan\""), Some("nan"));
        assert_eq!(model_name("other = 1"), None);
    }

    #[test]
    fn every_documented_variant_parses_and_an_unknown_one_does_not() {
        for name in ["linear", "friction-circle", "not-handled", "panics", "nan"] {
            assert!(Variant::parse(name).is_some(), "{name}");
        }
        assert_eq!(Variant::parse("quantum"), None);
    }
}
