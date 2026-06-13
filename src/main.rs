mod actions;
mod background;
mod capture;
mod capture_flow;
mod capture_gate;
mod desktop_layout;
mod i18n;
mod image_util;
mod model;
mod overlay;
mod preview;
mod screencast;
mod inline_text;
mod tray;
mod util;

use gpui::{App, Application};
use background::attach_background_window_guard;
use capture_gate::CaptureGateSession;
use overlay::{register_overlay_keybindings, CaptureBusy, CaptureInProgress, OverlaySession};
use tray::{attach_tray_to_app, spawn_tray};

fn main() {
    crate::util::log_capture_environment();

    let tray_rx = spawn_tray();

    Application::new().run(|cx: &mut App| {
        i18n::init(cx);
        cx.set_global(OverlaySession::default());
        cx.set_global(CaptureGateSession(None));
        cx.set_global(CaptureInProgress::default());
        cx.set_global(CaptureBusy::default());
        register_overlay_keybindings(cx);
        attach_background_window_guard(cx);
        cx.on_action(|_: &actions::Quit, cx| cx.quit());
        attach_tray_to_app(cx, tray_rx);

        if std::env::var("UBUNTUSCREENSHOT_AUTO_CAPTURE").is_ok() {
            cx.defer(|cx| crate::capture_flow::start_capture(cx));
        }
    });
}
