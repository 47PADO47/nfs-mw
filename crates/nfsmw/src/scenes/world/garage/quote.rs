//! A cart is always compared against the setup installed at entry, never a preview.
use anyhow::{Context, Result, ensure};
use nfsmw_career::economy::shop::{self, Item, Quote};
use nfsmw_data::car::customization::Category;

use super::WorldScene;
use crate::garage::Selection;

impl WorldScene {
    pub(super) fn garage_quote(&self, selection: &Selection) -> Result<Quote> {
        self.garage_quote_detail(selection, true).map(|(quote, _)| quote)
    }

    fn garage_quote_detail(&self, selection: &Selection, check_cash: bool) -> Result<(Quote, Vec<Item>)> {
        let session = self.career.garage.as_ref().context("No garage session is open")?;
        let Some(handle) = session.handle else {
            crate::frontend::validate_garage_unlocks(&self.dir, &self.garage, &session.baseline, selection)?;
            return Ok((Quote::default(), Vec::new()));
        };
        ensure!(selection.car == session.baseline.car, "The shop cannot change the current career car");
        let entry = self.garage.cars.iter().find(|car| car.folder == selection.car).context("Car is unavailable")?;
        let catalog = entry.parts.as_ref().context("Part catalog is unavailable")?;
        catalog.validate(&selection.parts)?;
        selection.gauges.validate(catalog)?;
        let old = &session.baseline;
        ensure!(
            selection.color == old.color
                || selection.color.is_none()
                || selection.parts.parts[Category::Paint as usize].is_some(),
            "Choose an installed paint finish before purchasing a custom body color"
        );
        let pricing =
            self.garage.pricing.clone().map(Ok).unwrap_or_else(|| crate::garage::pricing::Pricing::load(&self.dir))?;
        let free_junkman =
            !entry.hero && self.career.handle.with(|career| career.career_completed_once()).unwrap_or(false);
        let items = pricing.items(entry, old, selection, self.garage.backroom, free_junkman)?;
        let quote = self
            .career
            .handle
            .with(|career| -> Result<Quote> {
                ensure!(career.settings.car_handle == handle, "The career car changed while the garage was open");
                let mut settings = career.settings.clone();
                if !check_cash {
                    // The original keeps prices visible when the player cannot afford the cart.
                    settings.cash = u32::MAX;
                }
                if career.career_completed_once() {
                    settings.flags.insert(nfsmw_career::settings::Flags::CAREER_COMPLETE);
                }
                Ok(shop::quote(&settings, &career.markers, &items)?)
            })
            .context("The career is no longer active")??;
        Ok((quote, items))
    }

    pub(super) fn refresh_garage_quote(&mut self, selection: &Selection) {
        self.garage.shop_cash = self.career.handle.with(|career| career.cash());
        self.garage.shop_marker_counts = self
            .career
            .handle
            .with(|career| std::array::from_fn(|kind| career.markers.count_entries(kind as u32, 0)))
            .unwrap_or_default();
        self.garage.shop_markers =
            self.career.handle.is_active().then(|| self.garage.shop_marker_counts[1..].iter().sum());
        let dir = &self.dir;
        let table = *self.career.cooling.get_or_insert_with(|| super::super::career::cooling::read_table(dir));
        self.garage.shop_heat = self
            .career
            .handle
            .with(|career| {
                let heat = career.stable.career_for_handle(career.settings.car_handle)?.heat;
                let table = table?;
                let baseline = &self.career.garage.as_ref()?.baseline;
                let fitted = super::super::career::cooling::fitted_parts(baseline, selection);
                let extra = match self.garage.backroom {
                    true => nfsmw_career::economy::cooling::BACKROOM_EXTRA,
                    false => nfsmw_career::economy::cooling::SHOP_EXTRA,
                };
                Some((heat, nfsmw_career::economy::cooling::preview_heat(heat, &table, &fitted, extra)))
            })
            .flatten();
        match self.garage_quote_detail(selection, false) {
            Ok((quote, items)) => {
                self.garage.selection_locked = false;
                self.garage.shop_items = items;
                self.garage.shop_refund = Some(quote.refund);
                self.garage.shop_total = Some(match self.garage.backroom {
                    true => quote.markers.len() as u32,
                    false => quote.total,
                });
                self.garage.shop_error = self
                    .garage
                    .shop_cash
                    .filter(|cash| *cash < quote.total)
                    .map(|cash| shop::ShopError::NotEnoughCash { total: quote.total, cash }.to_string());
            }
            Err(error) => {
                self.garage.selection_locked = matches!(
                    error.downcast_ref::<nfsmw_career::economy::shop::ShopError>(),
                    Some(nfsmw_career::economy::shop::ShopError::Locked { .. })
                );
                self.garage.shop_items.clear();
                self.garage.shop_refund = None;
                self.garage.shop_total = None;
                self.garage.shop_error = Some(format!("{error:#}"));
            }
        }
    }
}
