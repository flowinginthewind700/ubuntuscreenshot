use crate::desktop_layout::VirtualDesktop;
use crate::util::{debug_log, desktop_app_id};
use gpui::{
    AnyWindowHandle, App, Bounds, Context, Global, Window, WindowBackgroundAppearance,
    WindowBounds, WindowOptions, div, point, prelude::*, px, size,
};
use std::sync::mpsc;

pub struct CaptureGateSession(pub Option<AnyWindowHandle>);

impl Global for CaptureGateSession {}

struct CaptureGate {
    ready_tx: Option<mpsc::Sender<()>>,
    fired: bool,
}

impl CaptureGate {
    fn fire(&mut self) {
        if self.fired {
            return;
        }
        self.fired = true;
        if let Some(tx) = self.ready_tx.take() {
            let _ = tx.send(());
        }
    }
}

impl Render for CaptureGate {
    fn render(&mut self, window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        window.activate_window();
        if window.is_window_active() {
            self.fire();
        }
        div().size_full()
    }
}

/// 托盘启动时先获得前台焦点，满足 portal「仅前台应用可弹授权框」。
pub fn open_capture_gate(cx: &mut App) -> mpsc::Receiver<()> {
    let (tx, rx) = mpsc::channel();
    cx.activate(true);

    let layout = VirtualDesktop::detect();
    let (x, y) = layout
        .as_ref()
        .and_then(|l| l.primary_monitor())
        .map(|m| (m.x, m.y))
        .unwrap_or((0, 0));

    debug_log(&format!(
        "capture_gate: opening focused window at ({x},{y}) app_id={}",
        desktop_app_id()
    ));

    if let Ok(handle) = cx.open_window(
        WindowOptions {
            titlebar: None,
            window_bounds: Some(WindowBounds::Windowed(Bounds::new(
                point(px(x as f32), px(y as f32)),
                size(px(1.), px(1.)),
            ))),
            window_background: WindowBackgroundAppearance::Transparent,
            window_decorations: None,
            focus: true,
            show: true,
            is_movable: false,
            is_resizable: false,
            kind: gpui::WindowKind::Normal,
            app_id: Some(desktop_app_id().into()),
            ..Default::default()
        },
        |_, cx| {
            cx.new(|_| CaptureGate {
                ready_tx: Some(tx),
                fired: false,
            })
        },
    ) {
        cx.set_global(CaptureGateSession(Some(handle.into())));
    }

    rx
}

pub fn close_capture_gate(cx: &mut App) {
    if let Some(handle) = cx
        .try_global::<CaptureGateSession>()
        .and_then(|g| g.0)
    {
        cx.defer(move |cx| {
            let _ = handle.update(cx, |_, window, _| window.remove_window());
        });
    }
    cx.set_global(CaptureGateSession(None));
}
