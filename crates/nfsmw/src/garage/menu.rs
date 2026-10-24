//! Transaction requests shared by the original game screens and the garage scenes.

use super::{Model, Selection};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MenuEvent {
    Preview(Selection),
    Confirm(Selection),
    Cancel,
    SelectSlot(String),
    AddSlot,
    DeleteSlot,
}

pub(crate) fn can_confirm(model: &Model) -> bool {
    if model.selected().is_none_or(|car| car.unavailable.is_some()) {
        return false;
    }
    if model.shop_error.is_some() {
        return false;
    }
    if !model.shop {
        return true;
    }
    model.shop_total.zip(model.shop_cash).is_some_and(|(total, cash)| model.backroom || total <= cash)
}
