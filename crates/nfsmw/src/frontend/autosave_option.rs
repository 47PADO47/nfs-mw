//! The native profile option is separate from global renderer/input settings.
use anyhow::{Context, Result};

use super::{Data, message, on_off};
use crate::career::{CareerHandle, ProfileDir};
use crate::frontend::confirm::Alert;
use crate::frontend::logic::{Command, Cx};
use crate::i18n::t;
use crate::ui::Text;

pub(super) fn data() -> Data {
    CareerHandle::shared()
        .with_loaded(|career| on_off(career.autosave.option_on))
        .unwrap_or_else(|| message("options-unavailable"))
}

pub(in crate::frontend) fn change(cx: &mut Cx) {
    if !CareerHandle::shared().is_loaded() {
        return;
    }
    let result: Result<()> = (|| {
        let profiles = ProfileDir::default_dir().context("no profile directory for Autosave preference")?;
        CareerHandle::shared()
            .with_loaded(|career| career.set_autosave_enabled(&profiles, !career.autosave.option_on))
            .context("the loaded profile changed")?
    })();
    if let Err(error) = result {
        log::warn!("Autosave preference not saved: {error:#}");
        cx.send(Command::Alert(Alert::new(Text::Literal(t!("options-autosave-save-failed")))));
    }
}
