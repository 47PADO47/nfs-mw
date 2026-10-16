//! `hello`: the smallest mod. It logs when it loads, counts the events it receives and logs each one, and logs when it
//! unloads. It asks for no interface, so it runs in any host that offers a `*` game.

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

    fn event(&mut self, ctx: &mut Ctx, event: &Event) {
        self.seen += 1;
        ctx.log(
            Level::Info,
            &format!("hello: event {} at frame {} (count {})", event.kind(), event.frame(), self.seen),
        );
    }

    fn unload(&mut self, ctx: &mut Ctx) {
        ctx.log(Level::Info, &format!("hello: unloaded after {} events", self.seen));
    }
}

declare_mod!(Hello);
