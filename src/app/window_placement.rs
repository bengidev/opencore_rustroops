//! Native window placement helpers where GPUI does not expose reposition APIs.

use gpui::{App, Bounds, Pixels, Size, Window};

/// Re-centers the window on the primary display after a programmatic resize.
pub fn center_window(window: &Window, size: Size<Pixels>, cx: &App) {
    #[cfg(target_os = "macos")]
    center_window_macos(window, size, cx);
    #[cfg(not(target_os = "macos"))]
    let _ = (window, size, cx);
}

#[cfg(target_os = "macos")]
fn center_window_macos(window: &Window, size: Size<Pixels>, cx: &App) {
    use objc::{class, msg_send, sel, sel_impl};
    use raw_window_handle::RawWindowHandle;

    let Some(display) = cx.primary_display() else {
        return;
    };
    let bounds = Bounds::centered(None, size, cx);

    let Ok(handle) = raw_window_handle::HasWindowHandle::window_handle(window) else {
        return;
    };
    let RawWindowHandle::AppKit(appkit) = handle.as_raw() else {
        return;
    };

    #[repr(C)]
    struct NSPoint {
        x: f64,
        y: f64,
    }

    #[repr(C)]
    struct NSRect {
        origin: NSPoint,
        size: NSPoint,
    }

    unsafe {
        let native_view = appkit.ns_view.as_ptr() as *mut objc::runtime::Object;
        let native_window: *mut objc::runtime::Object = msg_send![native_view, window];
        if native_window.is_null() {
            return;
        }

        // Primary display screen frame — matches gpui_macos window creation (screens[0]).
        let screens: *mut objc::runtime::Object = msg_send![class!(NSScreen), screens];
        let screen: *mut objc::runtime::Object = msg_send![screens, objectAtIndex: 0usize];
        if screen.is_null() {
            return;
        }

        let screen_frame: NSRect = msg_send![screen, frame];
        let top_left = NSPoint {
            x: screen_frame.origin.x + bounds.origin.x.as_f32() as f64,
            y: screen_frame.origin.y
                + (display.bounds().size.height - bounds.origin.y).as_f32() as f64,
        };

        let _: () = msg_send![native_window, setFrameTopLeftPoint: top_left];
    }
}
