//! A hidden Win32 surface exercises Streamline's mandatory presentation hooks.
use raw_window_handle::{
    DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, Win32WindowHandle, WindowHandle,
};
use std::num::NonZeroIsize;
use std::sync::Arc;

unsafe extern "C" {
    fn bb_sl_test_instance() -> isize;
    fn bb_sl_test_window_create() -> isize;
    fn bb_sl_test_window_resize(window: isize, width: u32, height: u32);
    fn bb_sl_test_window_visible(window: isize);
    fn bb_sl_test_window_pump();
    fn bb_sl_test_window_destroy(window: isize);
}

#[derive(Debug)]
pub(super) struct TestWindow(NonZeroIsize);

impl HasWindowHandle for TestWindow {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        // SAFETY: the window was created on this test thread and outlives this borrow.
        let mut handle = Win32WindowHandle::new(self.0);
        handle.hinstance = NonZeroIsize::new(unsafe { bb_sl_test_instance() });
        Ok(unsafe { WindowHandle::borrow_raw(handle.into()) })
    }
}

impl HasDisplayHandle for TestWindow {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        Ok(DisplayHandle::windows())
    }
}

impl TestWindow {
    pub(super) fn renderer() -> (Arc<Self>, crate::Renderer) {
        // SAFETY: the helper creates an owned, hidden window on the calling thread.
        let window = Arc::new(Self(NonZeroIsize::new(unsafe { bb_sl_test_window_create() }).expect("test window")));
        let renderer =
            crate::Renderer::new(window.clone(), (640, 360), window.clone(), super::super::test_support::options())
                .expect("native window renderer");
        (window, renderer)
    }
    pub(super) fn resize(&self, size: (u32, u32)) {
        // SAFETY: the window is alive and owned by this thread.
        unsafe { bb_sl_test_window_resize(self.0.get(), size.0, size.1) };
    }
    pub(super) fn visible(&self) {
        // SAFETY: the owned window is shown and activated for real presentation tests.
        unsafe { bb_sl_test_window_visible(self.0.get()) };
    }
    pub(super) fn pump(&self) {
        // SAFETY: dispatch messages on the window's owning test thread.
        unsafe { bb_sl_test_window_pump() };
    }
}

impl Drop for TestWindow {
    fn drop(&mut self) {
        // SAFETY: the last surface/display owner releases the window on its creating thread.
        unsafe { bb_sl_test_window_destroy(self.0.get()) };
    }
}
