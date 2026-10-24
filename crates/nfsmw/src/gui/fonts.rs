//! egui's fonts follow the language: a language pack's fallback fonts are added after egui's own, so subtitles and mod
//! messages in a script egui's fonts lack still draw (`docs/specs/localization.md` section 5).

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

/// The language generation the fonts were set for; `u64::MAX` before the first frame.
static APPLIED: AtomicU64 = AtomicU64::new(u64::MAX);

pub(super) fn follow_language(ctx: &egui::Context) {
    let now = crate::i18n::generation();
    if APPLIED.swap(now, Ordering::AcqRel) == now {
        return;
    }
    let mut definitions = egui::FontDefinitions::default();
    for (index, path) in crate::i18n::current().fonts.iter().enumerate() {
        let bytes = match std::fs::read(path) {
            Ok(bytes) => bytes,
            Err(e) => {
                log::warn!("{}: {e}", path.display());
                continue;
            }
        };
        let name = format!("language-{index}");
        definitions.font_data.insert(name.clone(), Arc::new(egui::FontData::from_owned(bytes)));
        for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
            definitions.families.entry(family).or_default().push(name.clone());
        }
    }
    ctx.set_fonts(definitions);
}
