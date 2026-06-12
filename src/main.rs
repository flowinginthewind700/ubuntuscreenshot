mod actions;
mod background;
mod capture;
mod desktop_layout;
mod capture_flow;
mod i18n;
mod image_util;
mod model;
mod overlay;
mod preview;
mod inline_text;
mod tray;
mod util;

use gpui::{App, Application};
use background::attach_background_window_guard;
use overlay::{register_overlay_keybindings, CaptureInProgress, OverlaySession};
use tray::{attach_tray_to_app, spawn_tray};

fn main() {
    let tray_rx = spawn_tray();

    Application::new().run(|cx: &mut App| {
        i18n::init(cx);
        cx.set_global(OverlaySession::default());
        cx.set_global(CaptureInProgress::default());
        register_overlay_keybindings(cx);
        attach_background_window_guard(cx);
        cx.on_action(|_: &actions::Quit, cx| cx.quit());
        attach_tray_to_app(cx, tray_rx);
    });
}
