use crate::capture::capture_primary_screen;
use crate::overlay::open_overlay;
use gpui::{
    App, Context, KeyBinding, SharedString, Timer, Window, WindowDecorations, WindowHandle,
    WindowOptions, div, prelude::*, px, rgb, rgba,
};
use std::time::Duration;

const BG: u32 = 0x0f1117;
const PANEL: u32 = 0x171b24;
const ACCENT: u32 = 0x007AFF;
const TEXT: u32 = 0xe8edf7;
const MUTED: u32 = 0x7d8da6;

pub struct LauncherApp {
    status: SharedString,
    capturing: bool,
}

impl LauncherApp {
    pub fn new(_cx: &mut Context<Self>) -> Self {
        Self {
            status: "就绪 · 点击开始截屏".into(),
            capturing: false,
        }
    }

    pub fn start_capture(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.capturing {
            return;
        }
        self.capturing = true;
        self.status = "正在截屏…".into();
        cx.notify();

        window.remove_window();

        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            // 与 capture_flow.rs 保持一致：为 zbus blocking API 提供 Tokio runtime
            // 上下文，但把同步截屏逻辑放在 spawn_blocking 中，避免嵌套 block_on。
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("failed to create tokio runtime for capture");

            rt.block_on(async {
                tokio::time::sleep(Duration::from_millis(180)).await;
                let result = tokio::task::spawn_blocking(capture_primary_screen)
                    .await
                    .unwrap_or_else(|e| Err(anyhow::anyhow!("capture panicked: {e}")));
                let _ = tx.send(result);
            });
        });

        cx.spawn(async move |_, cx| {
            loop {
                if let Ok(frame) = rx.try_recv() {
                    match frame {
                        Ok(frame) => {
                            let _ = cx.update(|cx| {
                                open_overlay(frame, cx);
                                cx.activate(true);
                            });
                        }
                        Err(e) => {
                            let _ = cx.update(|cx| {
                                crate::reopen_launcher(cx);
                            });
                            eprintln!("截屏失败: {e}");
                        }
                    }
                    break;
                }
                Timer::after(Duration::from_millis(50)).await;
            }
        })
        .detach();
    }
}

fn primary_button(
    label: &'static str,
    cx: &mut Context<LauncherApp>,
    on_click: impl Fn(&mut LauncherApp, &mut Window, &mut Context<LauncherApp>) + 'static,
) -> impl IntoElement {
    div()
        .id(SharedString::from(label))
        .w_full()
        .px_4()
        .py_3()
        .rounded_lg()
        .bg(rgb(ACCENT))
        .text_color(rgb(0xffffff))
        .text_sm()
        .font_weight(gpui::FontWeight::SEMIBOLD)
        .cursor_pointer()
        .flex()
        .items_center()
        .justify_center()
        .hover(|this| this.opacity(0.9))
        .active(|this| this.opacity(0.82))
        .child(label.to_string())
        .on_click(cx.listener(move |this, _, window, cx| on_click(this, window, cx)))
}

impl Render for LauncherApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let status = self.status.clone();

        div()
            .size_full()
            .bg(rgb(BG))
            .flex()
            .flex_col()
            .p_6()
            .gap_4()
            .key_context("Launcher")
            .on_action(cx.listener(|this, _: &crate::actions::StartCapture, window, cx| {
                this.start_capture(window, cx);
            }))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        div()
                            .text_xl()
                            .font_weight(gpui::FontWeight::BOLD)
                            .text_color(rgb(TEXT))
                            .child("ubuntuscreenshot"),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(rgb(MUTED))
                            .child("微信风格截屏 · 框选 · 标注 · 保存/复制"),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .justify_center()
                    .gap_3()
                    .child(
                        div()
                            .p_5()
                            .rounded_xl()
                            .bg(rgb(PANEL))
                            .border_1()
                            .border_color(rgba(0xffffff10))
                            .flex()
                            .flex_col()
                            .gap_4()
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(rgb(MUTED))
                                    .child("截屏时会自动隐藏此窗口"),
                            )
                            .child(primary_button("开始截屏", cx, |this, window, cx| {
                                this.start_capture(window, cx);
                            })),
                    ),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(rgb(MUTED))
                    .child(status),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(rgb(MUTED))
                    .child("Print Screen / Ctrl+Shift+A  开始 · Esc 取消"),
            )
    }
}

pub fn open_launcher(cx: &mut App) -> WindowHandle<LauncherApp> {
    let bounds = gpui::Bounds::centered(None, gpui::size(px(380.0), px(300.0)), cx);
    let handle = cx
        .open_window(
            WindowOptions {
                titlebar: Some(gpui::TitlebarOptions {
                    title: Some("ubuntuscreenshot".into()),
                    ..Default::default()
                }),
                window_bounds: Some(gpui::WindowBounds::Windowed(bounds)),
                window_decorations: Some(WindowDecorations::Server),
                ..Default::default()
            },
            |_, cx| cx.new(LauncherApp::new),
        )
        .expect("无法打开启动窗口");

    cx.bind_keys([
        KeyBinding::new("printscreen", crate::actions::StartCapture, Some("Launcher")),
        KeyBinding::new("ctrl-shift-a", crate::actions::StartCapture, Some("Launcher")),
        KeyBinding::new("ctrl-q", crate::actions::Quit, None),
    ]);

    handle
}

pub fn register_global_actions(cx: &mut App) {
    cx.on_action(|_: &crate::actions::Quit, cx| cx.quit());
}
