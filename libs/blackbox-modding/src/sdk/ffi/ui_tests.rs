//! The SDK's `Ui` wrapper, round-tripped through the host's `bb.ui` table: a theme and its slot reach a fake game backend
//! with every primitive kind intact, and unregister reaches the backend. The host side runs in a callback scope.

use std::cell::RefCell;
use std::rc::Rc;

use super::ui as sdk_ui;
use crate::abi::{IFACE_UI, IFACE_UI_VERSION};
use crate::host::test_support::callback_on_main;
use crate::host::{Attached, BackendResult, InterfaceRegistry, UiBackend, UiPrim, UiRect, UiThemeSpec};
use crate::sdk::ui::{Prim, Rect};
use crate::sdk::{Interface, Ui};

#[derive(Default)]
struct FakeUi {
    themes: RefCell<Vec<(u64, String)>>,
    slots: RefCell<Vec<(u64, u32, Vec<UiPrim>)>>,
    removed: RefCell<Vec<u64>>,
}

impl UiBackend for FakeUi {
    fn register_theme(&self, _owner: &str, theme: &UiThemeSpec<'_>) -> BackendResult<u64> {
        let handle = crate::abi::make_handle(self.themes.borrow().len() as u32 + 1, 1);
        self.themes.borrow_mut().push((handle, theme.name.to_owned()));
        Ok(handle)
    }

    fn set_slot(&self, _owner: &str, theme: u64, slot: u32, prims: &[UiPrim]) -> BackendResult<()> {
        self.slots.borrow_mut().push((theme, slot, prims.to_vec()));
        Ok(())
    }

    fn unregister_theme(&self, _owner: &str, theme: u64) -> BackendResult<()> {
        self.removed.borrow_mut().push(theme);
        Ok(())
    }
}

/// The SDK's `Ui` over a registry that offers `backend` as `bb.ui`, for mod `example.ui`. The attachment is returned too:
/// it owns the host's table, so the caller keeps it alive for the test.
fn ui_over(backend: &Rc<FakeUi>) -> (Ui, Attached) {
    let mut registry = InterfaceRegistry::new();
    registry.add_ui(backend.clone());
    let attached = registry.build("example.ui", IFACE_UI, IFACE_UI_VERSION.0, 0).expect("bb.ui is offered");
    let header = attached.header();
    // SAFETY: `attached` owns the table, and the header it points at is a readable `BbIface` for the test's lifetime.
    let header_value = unsafe { *header };
    let iface = Interface::new(core::ptr::NonNull::new(header.cast_mut()).expect("non-null header"), header_value);
    (Ui::new(sdk_ui::read(iface)), attached)
}

#[test]
fn a_theme_and_its_slot_reach_the_backend_intact() {
    let _scope = callback_on_main();
    let backend = Rc::new(FakeUi::default());
    let (ui, _attached) = ui_over(&backend);
    let theme = ui.register_theme("night", "Night drive").expect("register");
    let rect = Rect { x: 1.0, y: -2.0, w: 30.0, h: 40.0 };
    let prims = [
        Prim::Panel { rect, colour: [9, 8, 7, 255], texture: Some("Panel_Back") },
        Prim::Text { rect, colour: [255; 4], justify: 1, font: "FONT_MW_BODY", text: "SPEED" },
        Prim::Image { rect, uv: [0.0, 0.0, 1.0, 0.5], texture: "Outrun_Backing" },
    ];
    ui.set_slot(theme, 4, &prims).expect("set slot");

    assert_eq!(*backend.themes.borrow(), [(theme.raw(), "night".to_owned())]);
    let slots = backend.slots.borrow();
    let r = UiRect { x: 1.0, y: -2.0, w: 30.0, h: 40.0 };
    assert_eq!(slots[0].0, theme.raw());
    assert_eq!(slots[0].1, 4);
    assert_eq!(
        slots[0].2,
        [
            UiPrim::Panel { rect: r, colour: [9, 8, 7, 255], texture: Some("Panel_Back".into()) },
            UiPrim::Text { rect: r, colour: [255; 4], justify: 1, font: "FONT_MW_BODY".into(), text: "SPEED".into() },
            UiPrim::Image { rect: r, uv: [0.0, 0.0, 1.0, 0.5], texture: "Outrun_Backing".into() },
        ]
    );
    drop(slots);
    ui.unregister_theme(theme).expect("unregister");
    assert_eq!(*backend.removed.borrow(), [theme.raw()]);
}
