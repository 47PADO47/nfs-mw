//! The theme registry: the Original floor, the other registered themes, the active one, and the resolver.
//!
//! `resolve` asks the active theme for a slot. A `None` from it falls back to Original. Original is a
//! contract to provide every slot the UI uses, so the resolver only falls through to `Empty` when that
//! contract is broken: the slot then draws nothing, and nothing panics.

use thiserror::Error;

use crate::component::{Component, Empty};
use crate::context::BuildCx;
use crate::slot::SlotId;
use crate::theme::{Theme, ThemeId};

/// Why a registry call was refused.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ThemeError {
    #[error("the floor theme must have id Original, not {0:?}")]
    NotOriginal(ThemeId),
    #[error("theme {0:?} is already registered")]
    Duplicate(ThemeId),
    #[error("theme {0:?} is not registered")]
    Unknown(ThemeId),
}

/// The registered themes and the active choice.
pub struct Themes {
    original: Box<dyn Theme>,
    others: Vec<Box<dyn Theme>>,
    active: ThemeId,
}

impl Themes {
    /// Creates the registry with `original` as the floor and the active theme. Refuses any theme whose id is
    /// not `ThemeId::Original`.
    pub fn new(original: Box<dyn Theme>) -> Result<Self, ThemeError> {
        let id = original.id();
        if id != ThemeId::Original {
            return Err(ThemeError::NotOriginal(id));
        }
        Ok(Self { original, others: Vec::new(), active: ThemeId::Original })
    }

    /// Adds a theme that can be made active. Refuses an id already present, including `Original`.
    pub fn register(&mut self, theme: Box<dyn Theme>) -> Result<(), ThemeError> {
        let id = theme.id();
        if self.find(id).is_some() {
            return Err(ThemeError::Duplicate(id));
        }
        self.others.push(theme);
        Ok(())
    }

    /// Makes a registered theme active. Refuses an id that was never registered.
    pub fn set_active(&mut self, id: ThemeId) -> Result<(), ThemeError> {
        if self.find(id).is_none() {
            return Err(ThemeError::Unknown(id));
        }
        self.active = id;
        Ok(())
    }

    pub fn active(&self) -> ThemeId {
        self.active
    }

    /// Every registered theme id, Original first, then in registration order.
    pub fn ids(&self) -> impl Iterator<Item = ThemeId> + '_ {
        std::iter::once(self.original.id()).chain(self.others.iter().map(|t| t.id()))
    }

    /// The component for `slot`: the active theme's, else Original's, else `Empty`.
    pub fn resolve(&self, slot: SlotId, cx: &BuildCx<'_>) -> Box<dyn Component> {
        let from_active = self.find(self.active).and_then(|theme| theme.element(slot, cx));
        if let Some(component) = from_active {
            return component;
        }
        if let Some(component) = self.original.element(slot, cx) {
            return component;
        }
        Box::new(Empty)
    }

    /// The theme `resolve` takes the component for `slot` from: the active theme when it provides the slot, else
    /// Original, else `None` when no theme provides it. Asks each theme for a component, so it costs what `resolve`
    /// costs.
    pub fn provider(&self, slot: SlotId, cx: &BuildCx<'_>) -> Option<ThemeId> {
        let from_active = self.find(self.active).is_some_and(|theme| theme.element(slot, cx).is_some());
        if from_active {
            return Some(self.active);
        }
        self.original.element(slot, cx).map(|_| ThemeId::Original)
    }

    fn find(&self, id: ThemeId) -> Option<&dyn Theme> {
        if id == ThemeId::Original {
            return Some(self.original.as_ref());
        }
        self.others.iter().find(|theme| theme.id() == id).map(|theme| theme.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::{ThemeError, Themes};
    use crate::component::Component;
    use crate::context::{BuildCx, LayoutCx};
    use crate::draw::DrawList;
    use crate::geom::Rect;
    use crate::skin::Skin;
    use crate::slot::SlotId;
    use crate::theme::{Theme, ThemeId};
    use crate::tree::{NodeKind, UiNode};

    /// Draws one node whose red channel is the marker, so a test can see which theme produced it.
    struct Marked(u8);

    impl Component for Marked {
        fn layout(&mut self, _cx: &LayoutCx<'_>, _bounds: Rect) {}

        fn build(&self, out: &mut DrawList) {
            out.push(UiNode { kind: NodeKind::Group, colour: [self.0, 0, 0, 255], visible: true, ..UiNode::default() });
        }
    }

    /// A theme that provides the listed slots, each with its marker.
    struct TestTheme {
        id: ThemeId,
        slots: &'static [(SlotId, u8)],
    }

    impl Theme for TestTheme {
        fn id(&self) -> ThemeId {
            self.id
        }

        fn element(&self, slot: SlotId, _cx: &BuildCx<'_>) -> Option<Box<dyn Component>> {
            let marker = self.slots.iter().find(|(name, _)| *name == slot).map(|(_, marker)| *marker)?;
            Some(Box::new(Marked(marker)))
        }
    }

    const PANEL_A: SlotId = SlotId::new(1);
    const RADIO: SlotId = SlotId::new(2);
    const UNKNOWN: SlotId = SlotId::new(99);

    const ORIGINAL_SLOTS: &[(SlotId, u8)] = &[(PANEL_A, 10), (RADIO, 11)];
    const CUSTOM_SLOTS: &[(SlotId, u8)] = &[(RADIO, 20)];
    const MOD_SLOTS: &[(SlotId, u8)] = &[(RADIO, 30)];

    fn theme(id: ThemeId, slots: &'static [(SlotId, u8)]) -> Box<dyn Theme> {
        Box::new(TestTheme { id, slots })
    }

    fn registry() -> Themes {
        Themes::new(theme(ThemeId::Original, ORIGINAL_SLOTS)).expect("original floor")
    }

    /// The marker of the component `slot` resolves to, or `None` when the component draws nothing.
    fn resolved(themes: &Themes, slot: SlotId) -> Option<u8> {
        let skin = Skin::new();
        let component = themes.resolve(slot, &BuildCx::new(&skin));
        let mut out = DrawList::new();
        component.build(&mut out);
        out.iter().next().map(|node| node.colour[0])
    }

    #[test]
    fn resolver_returns_active_theme_component_when_it_provides_the_slot() {
        let mut themes = registry();
        themes.register(theme(ThemeId::Custom, CUSTOM_SLOTS)).expect("register custom");
        themes.set_active(ThemeId::Custom).expect("activate custom");
        assert_eq!(resolved(&themes, RADIO), Some(20));
    }

    #[test]
    fn resolver_falls_back_to_original_when_active_theme_returns_none() {
        let mut themes = registry();
        themes.register(theme(ThemeId::Custom, CUSTOM_SLOTS)).expect("register custom");
        themes.set_active(ThemeId::Custom).expect("activate custom");
        assert_eq!(resolved(&themes, PANEL_A), Some(10));
    }

    #[test]
    fn original_active_resolves_from_original() {
        let mut themes = registry();
        themes.register(theme(ThemeId::Custom, CUSTOM_SLOTS)).expect("register custom");
        assert_eq!(themes.active(), ThemeId::Original);
        assert_eq!(resolved(&themes, RADIO), Some(11));
    }

    #[test]
    fn slot_nobody_provides_draws_nothing() {
        let mut themes = registry();
        themes.register(theme(ThemeId::Custom, CUSTOM_SLOTS)).expect("register custom");
        themes.set_active(ThemeId::Custom).expect("activate custom");
        assert_eq!(resolved(&themes, UNKNOWN), None);
    }

    #[test]
    fn broken_floor_yields_empty_component_not_a_panic() {
        let themes = Themes::new(theme(ThemeId::Original, &[])).expect("original floor");
        assert_eq!(resolved(&themes, PANEL_A), None);
    }

    #[test]
    fn mod_theme_is_selectable_and_falls_back_for_other_slots() {
        let mut themes = registry();
        themes.register(theme(ThemeId::Mod(7), MOD_SLOTS)).expect("register mod");
        themes.set_active(ThemeId::Mod(7)).expect("activate mod");
        assert_eq!(resolved(&themes, RADIO), Some(30));
        assert_eq!(resolved(&themes, PANEL_A), Some(10));
    }

    #[test]
    fn floor_must_be_original() {
        let err = Themes::new(theme(ThemeId::Custom, CUSTOM_SLOTS)).err().expect("non-original floor refused");
        assert_eq!(err, ThemeError::NotOriginal(ThemeId::Custom));
    }

    #[test]
    fn register_refuses_duplicates_and_original() {
        let mut themes = registry();
        themes.register(theme(ThemeId::Custom, CUSTOM_SLOTS)).expect("first custom");
        let dup = themes.register(theme(ThemeId::Custom, CUSTOM_SLOTS));
        assert_eq!(dup, Err(ThemeError::Duplicate(ThemeId::Custom)));
        let again = themes.register(theme(ThemeId::Original, ORIGINAL_SLOTS));
        assert_eq!(again, Err(ThemeError::Duplicate(ThemeId::Original)));
    }

    #[test]
    fn set_active_refuses_unregistered_theme() {
        let mut themes = registry();
        let refused = themes.set_active(ThemeId::Mod(9));
        assert_eq!(refused, Err(ThemeError::Unknown(ThemeId::Mod(9))));
        assert_eq!(themes.active(), ThemeId::Original);
    }

    /// The theme `provider` names for `slot`.
    fn provided(themes: &Themes, slot: SlotId) -> Option<ThemeId> {
        let skin = Skin::new();
        themes.provider(slot, &BuildCx::new(&skin))
    }

    #[test]
    fn provider_names_the_theme_resolve_takes_the_slot_from() {
        let mut themes = registry();
        themes.register(theme(ThemeId::Custom, CUSTOM_SLOTS)).expect("register custom");
        themes.set_active(ThemeId::Custom).expect("activate custom");
        assert_eq!(provided(&themes, RADIO), Some(ThemeId::Custom));
        assert_eq!(provided(&themes, PANEL_A), Some(ThemeId::Original));
        assert_eq!(provided(&themes, UNKNOWN), None);
    }

    #[test]
    fn provider_names_original_when_the_original_theme_is_active() {
        let themes = registry();
        assert_eq!(provided(&themes, RADIO), Some(ThemeId::Original));
    }

    #[test]
    fn ids_lists_original_first_then_registration_order() {
        let mut themes = registry();
        themes.register(theme(ThemeId::Custom, CUSTOM_SLOTS)).expect("register custom");
        themes.register(theme(ThemeId::Mod(7), MOD_SLOTS)).expect("register mod");
        let ids: Vec<ThemeId> = themes.ids().collect();
        assert_eq!(ids, vec![ThemeId::Original, ThemeId::Custom, ThemeId::Mod(7)]);
    }
}
