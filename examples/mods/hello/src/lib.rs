//! `hello`: the smallest mod. It logs when it loads, counts the events it receives and logs each one, reads the clock on
//! its first event when the host offers `bb.time` (an optional interface: the mod says so and goes on when it is absent),
//! and logs when it unloads. It needs no other interface, so it runs in any host that offers a `*` game.
//!
//! The clock is read on the first event, not at load: the host copies the scene's clock in at the start of the first
//! frame, and a mod that reads at load (before any scene exists) gets "not ready".

use blackbox_modding::sdk::{Ctx, Event, Level, Mod, ModResult, declare_mod};

/// The mod's state: how many events it has seen.
pub struct Hello {
    seen: u64,
}

impl Mod for Hello {
    fn load(ctx: &mut Ctx) -> ModResult<Self> {
        ctx.log(Level::Info, "hello: loaded");
        Ok(Hello { seen: 0 })
    }

    fn event(&mut self, ctx: &mut Ctx, event: &Event<'_>) {
        self.seen += 1;
        ctx.log(
            Level::Info,
            &format!("hello: event {} at frame {} (count {})", event.kind(), event.frame(), self.seen),
        );
        if self.seen == 1 {
            log_clock(ctx);
        }
    }

    fn unload(&mut self, ctx: &mut Ctx) {
        ctx.log(Level::Info, &format!("hello: unloaded after {} events", self.seen));
    }
}

/// Log the time of day through `bb.time`. A host without the interface is not an error.
fn log_clock(ctx: &Ctx) {
    let Some(time) = ctx.time() else {
        ctx.log(Level::Info, "hello: bb.time absent");
        return;
    };
    match time.get() {
        Ok(clock) => ctx.log(
            Level::Info,
            &format!("hello: time {:.2} h, {} s per hour, frozen {}", clock.hour, clock.seconds_per_hour, clock.frozen),
        ),
        Err(error) => ctx.log(Level::Warn, &format!("hello: bb.time read failed: {error}")),
    }
}

declare_mod!(Hello);
