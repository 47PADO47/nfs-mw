//! The `bb.ui` table, called directly with a fake backend: a theme and its slot specs are read and checked, a bad spec is
//! refused before the backend sees it, and the themes a mod leaves behind are removed when it unloads.

use crate::host::test_support::callback_on_main;
use std::cell::RefCell;
use std::mem::size_of;
use std::ptr;
use std::rc::Rc;

use super::ui::{MAX_PRIMITIVES, UiBackend, UiPrim, UiRect, UiThemeSpec, UiTokens};
use super::{Attached, attach_ui};
use crate::abi::*;
use crate::host::BackendResult;
use crate::host::registry::InterfaceRegistry;

/// The table an attachment holds. The header is the first field of every interface table.
fn table_of(attached: &Attached) -> &BbUi {
    // SAFETY: the attachment owns a `BbUi` whose first field is the `BbIface` header (see `Attached::new`).
    unsafe { &*attached.header().cast::<BbUi>() }
}

/// A theme the backend was given, with its text copied out.
#[derive(Debug, Clone, PartialEq)]
struct StoredTheme {
    name: String,
    display: String,
}

#[derive(Default)]
struct FakeUi {
    themes: RefCell<Vec<(u64, StoredTheme)>>,
    slots: RefCell<Vec<(u64, u32, Vec<UiPrim>)>>,
    removed: RefCell<Vec<u64>>,
    tokens: RefCell<Vec<(u64, UiTokens)>>,
}

impl UiBackend for FakeUi {
    fn register_theme(&self, _owner: &str, theme: &UiThemeSpec<'_>) -> BackendResult<u64> {
        let handle = crate::abi::make_handle(self.themes.borrow().len() as u32 + 1, 1);
        let stored = StoredTheme { name: theme.name.to_owned(), display: theme.display.to_owned() };
        self.themes.borrow_mut().push((handle, stored));
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

    fn set_tokens(&self, _owner: &str, theme: u64, tokens: &UiTokens) -> BackendResult<()> {
        self.tokens.borrow_mut().push((theme, *tokens));
        Ok(())
    }
}

fn attached(backend: &Rc<FakeUi>) -> Attached {
    attach_ui(backend.clone(), "example.a")
}

/// A panel primitive with the fields the other kinds do not use set to zero.
fn panel() -> BbUiPrim {
    BbUiPrim {
        size: size_of::<BbUiPrim>() as u32,
        kind: BB_UI_PANEL,
        x: 10.0,
        y: -20.0,
        w: 100.0,
        h: 40.0,
        colour: [1, 2, 3, 255],
        justify: 0,
        uv: [0.0; 4],
        font: BbStr::EMPTY,
        texture: BbStr::EMPTY,
        text: BbStr::EMPTY,
    }
}

fn text(font: &str, string: &str) -> BbUiPrim {
    BbUiPrim { kind: BB_UI_TEXT, font: BbStr::new(font), text: BbStr::new(string), ..panel() }
}

fn image(texture: &str) -> BbUiPrim {
    BbUiPrim { kind: BB_UI_IMAGE, uv: [0.0, 0.0, 0.5, 1.0], texture: BbStr::new(texture), ..panel() }
}

/// Register a theme through the table; the status and the handle.
fn register_theme(table: &BbUi, name: &str, display: &str) -> (BbStatus, u64) {
    let mut id = 0;
    let register = table.register_theme.expect("register_theme is offered");
    // SAFETY: the table is the attachment's own; the strings and the out pointer are valid for the call.
    let status = unsafe { register(table.ctx, BbStr::new(name), BbStr::new(display), &mut id) };
    (status, id)
}

/// Give slot `slot` of `theme` the primitives `prims`; the status.
fn set_slot(table: &BbUi, theme: u64, slot: u32, prims: &[BbUiPrim]) -> BbStatus {
    let set = table.set_slot.expect("set_slot is offered");
    // SAFETY: the table is the attachment's own; `prims` is valid for `len` reads for the call.
    unsafe { set(table.ctx, theme, slot, prims.as_ptr(), prims.len() as u64) }
}

#[test]
fn a_theme_and_its_slot_reach_the_backend_with_every_primitive_kind() {
    let _scope = callback_on_main();
    let backend = Rc::new(FakeUi::default());
    let attach = attached(&backend);
    let table = table_of(&attach);
    let (status, theme) = register_theme(table, "night", "Night drive");
    assert_eq!(status, BB_OK);
    assert_ne!(theme, 0, "a non-zero handle");

    let prims = [panel(), text("FONT_MW_BODY", "SPEED"), image("Outrun_Backing")];
    assert_eq!(set_slot(table, theme, 4, &prims), BB_OK);

    let themes = backend.themes.borrow();
    assert_eq!(themes[0].1, StoredTheme { name: "night".into(), display: "Night drive".into() });
    let slots = backend.slots.borrow();
    assert_eq!(slots[0].0, theme);
    assert_eq!(slots[0].1, 4);
    let rect = UiRect { x: 10.0, y: -20.0, w: 100.0, h: 40.0 };
    assert_eq!(
        slots[0].2,
        [
            UiPrim::Panel { rect, colour: [1, 2, 3, 255], texture: None },
            UiPrim::Text {
                rect,
                colour: [1, 2, 3, 255],
                justify: 0,
                font: "FONT_MW_BODY".into(),
                text: "SPEED".into()
            },
            UiPrim::Image { rect, uv: [0.0, 0.0, 0.5, 1.0], texture: "Outrun_Backing".into() },
        ]
    );
}

#[test]
fn a_bad_spec_is_refused_before_the_backend_sees_it() {
    let _scope = callback_on_main();
    let backend = Rc::new(FakeUi::default());
    let attach = attached(&backend);
    let table = table_of(&attach);
    let (_, theme) = register_theme(table, "night", "Night");
    let refused = |prims: &[BbUiPrim]| set_slot(table, theme, 4, prims);
    assert_eq!(refused(&[BbUiPrim { kind: 9, ..panel() }]), BB_ERR_ARG, "an unknown kind");
    assert_eq!(refused(&[BbUiPrim { size: 8, ..panel() }]), BB_ERR_ARG, "a size too small for the primitive");
    assert_eq!(refused(&[BbUiPrim { x: f32::NAN, ..panel() }]), BB_ERR_ARG, "a non-finite position");
    assert_eq!(refused(&[BbUiPrim { w: -1.0, ..panel() }]), BB_ERR_ARG, "a negative width");
    assert_eq!(refused(&[BbUiPrim { uv: [0.0, f32::INFINITY, 1.0, 1.0], ..image("x") }]), BB_ERR_ARG, "a bad uv");
    assert_eq!(refused(&[BbUiPrim { justify: 0x20, ..text("", "a") }]), BB_ERR_ARG, "justification bits out of range");
    assert_eq!(refused(&[image("")]), BB_ERR_ARG, "an image without a texture");
    assert_eq!(refused(&[]), BB_ERR_ARG, "a slot with no primitives");
    let many = vec![panel(); MAX_PRIMITIVES + 1];
    assert_eq!(refused(&many), BB_ERR_ARG, "more primitives than the host takes");
    let null = {
        let set = table.set_slot.expect("set_slot is offered");
        // SAFETY: a null list with a count of 1 is refused before any read.
        unsafe { set(table.ctx, theme, 4, ptr::null(), 1) }
    };
    assert_eq!(null, BB_ERR_ARG, "a null list is refused");
    assert!(backend.slots.borrow().is_empty(), "nothing reached the backend");
}

#[test]
fn a_theme_left_registered_is_removed_when_its_mod_unloads() {
    let _scope = callback_on_main();
    let backend = Rc::new(FakeUi::default());
    let attach = attached(&backend);
    let table = table_of(&attach);
    let (_, left) = register_theme(table, "one", "One");
    let (_, gone) = register_theme(table, "two", "Two");
    let unregister = table.unregister_theme.expect("unregister_theme is offered");
    // SAFETY: the table is the attachment's own.
    assert_eq!(unsafe { unregister(table.ctx, gone) }, BB_OK);
    drop(attach);
    assert_eq!(*backend.removed.borrow(), [gone, left], "the explicit removal, then the unload's");
}

#[test]
fn a_registry_offers_the_ui_interface_at_its_version() {
    let _scope = callback_on_main();
    let backend: Rc<dyn UiBackend> = Rc::new(FakeUi::default());
    let mut registry = InterfaceRegistry::new();
    registry.add_ui(backend);
    assert!(registry.build("example.a", IFACE_UI, 1, 0).is_some());
    assert!(registry.build("example.a", IFACE_UI, 1, 1).is_none(), "a newer minor is not offered");
    assert!(registry.build("example.a", IFACE_UI, 2, 0).is_none(), "another major is not offered");
}
