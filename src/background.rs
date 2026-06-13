use crate::util::desktop_app_id;
use gpui::{
    App, Bounds, Context, Window, WindowBounds, WindowOptions, div, point, prelude::*, px, size,
};

/// Hidden window to keep GPUI event loop running for tray-only mode.
struct BackgroundApp;

impl Render for BackgroundApp {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().size_full()
    }
}

pub fn open_background_window(cx: &mut App) {
    if !cx.windows().is_empty() {
        return;
    }

    cx.open_window(
        WindowOptions {
            titlebar: None,
            window_bounds: Some(WindowBounds::Windowed(Bounds::new(
                point(px(0.), px(0.)),
                size(px(1.), px(1.)),
            ))),
            window_decorations: None,
            show: false,
            focus: false,
            is_movable: false,
            is_resizable: false,
            app_id: Some(desktop_app_id().into()),
            ..Default::default()
        },
        |_, cx| cx.new(|_| BackgroundApp),
    )
    .ok();
}

pub fn attach_background_window_guard(cx: &mut App) {
    open_background_window(cx);
    cx.on_window_closed(|cx| {
        if cx.windows().is_empty() {
            open_background_window(cx);
        }
    });
}
