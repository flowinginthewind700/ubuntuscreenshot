use crate::i18n::{self, Language, MessageKey, format_one, tr};
use crate::image_util::{copy_image_to_clipboard, frame_to_gpui_image, save_png};
use gpui::{
    App, Bounds, Context, KeyBinding, SharedString, Window, WindowBackgroundAppearance,
    WindowBounds, WindowOptions, canvas, div, point, prelude::*, px, rgb, size,
};
use image::RgbaImage;
use std::sync::Arc;

#[derive(Clone, Debug)]
enum PreviewStatus {
    Ready,
    Copied,
    CopyFailed(String),
    SaveCancelled,
    Saved(String),
    SaveFailed(String),
    DialogFailed(String),
}

impl PreviewStatus {
    fn message(&self, lang: Language) -> String {
        match self {
            Self::Ready => tr(lang, MessageKey::PreviewReady).into(),
            Self::Copied => tr(lang, MessageKey::PreviewCopied).into(),
            Self::CopyFailed(e) => format_one(
                lang,
                tr(lang, MessageKey::PreviewCopyFailed),
                "{error}",
                e,
            ),
            Self::SaveCancelled => tr(lang, MessageKey::PreviewSaveCancelled).into(),
            Self::Saved(path) => format_one(
                lang,
                tr(lang, MessageKey::PreviewSaved),
                "{path}",
                path,
            ),
            Self::SaveFailed(e) => format_one(
                lang,
                tr(lang, MessageKey::PreviewSaveFailed),
                "{error}",
                e,
            ),
            Self::DialogFailed(e) => format_one(
                lang,
                tr(lang, MessageKey::PreviewDialogFailed),
                "{error}",
                e,
            ),
        }
    }
}

pub struct PreviewApp {
    image: Arc<gpui::Image>,
    rgba: RgbaImage,
    status: PreviewStatus,
    locale_bound: bool,
}

impl PreviewApp {
    fn new(rgba: RgbaImage, _cx: &mut Context<Self>) -> Self {
        let image = Arc::new(
            frame_to_gpui_image(&crate::model::CaptureFrame {
                width: rgba.width(),
                height: rgba.height(),
                origin_x: 0,
                origin_y: 0,
                pixels: rgba.as_raw().to_vec(),
            })
            .expect("预览图转换失败"),
        );
        Self {
            image,
            rgba,
            status: PreviewStatus::Ready,
            locale_bound: false,
        }
    }

    fn copy(&mut self, cx: &mut Context<Self>) {
        match copy_image_to_clipboard(&self.rgba) {
            Ok(()) => self.status = PreviewStatus::Copied,
            Err(e) => self.status = PreviewStatus::CopyFailed(e.to_string()),
        }
        cx.notify();
    }

    fn save(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let lang = i18n::language(cx);
        let default_dir = crate::image_util::default_save_dir();
        let default_name = chrono::Local::now()
            .format("Screenshot_%Y%m%d_%H%M%S.png")
            .to_string();
        let default_path = default_dir.join(&default_name);

        let output = std::process::Command::new("zenity")
            .arg("--file-selection")
            .arg("--save")
            .arg("--confirm-overwrite")
            .arg(format!(
                "--file-filter={}",
                tr(lang, MessageKey::ZenityPngFilter)
            ))
            .arg(format!("--filename={}", default_path.display()))
            .arg(format!(
                "--title={}",
                tr(lang, MessageKey::ZenitySaveTitle)
            ))
            .output();

        match output {
            Ok(output) if output.status.success() => {
                let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if path_str.is_empty() {
                    self.status = PreviewStatus::SaveCancelled;
                } else {
                    match save_png(&self.rgba, std::path::Path::new(&path_str)) {
                        Ok(()) => {
                            self.status = PreviewStatus::Saved(path_str);
                        }
                        Err(e) => {
                            self.status = PreviewStatus::SaveFailed(e.to_string());
                        }
                    }
                }
            }
            Ok(_) => {
                self.status = PreviewStatus::SaveCancelled;
            }
            Err(e) => {
                self.status = PreviewStatus::DialogFailed(e.to_string());
            }
        }
        cx.notify();
    }

    fn close(&mut self, window: &mut Window) {
        window.remove_window();
    }
}

impl Render for PreviewApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.locale_bound {
            self.locale_bound = true;
            cx.observe_global_in::<i18n::LocaleSettings>(window, |_, _, cx| cx.notify())
                .detach();
        }

        let lang = i18n::language(cx);
        window.set_window_title(tr(lang, MessageKey::PreviewTitle));

        let gpui_image = self.image.clone();
        let status = self.status.message(lang);
        let img_w = self.rgba.width() as f32;
        let img_h = self.rgba.height() as f32;
        let max_w = 960.0;
        let scale = if img_w > max_w { max_w / img_w } else { 1.0 };
        let preview_w = img_w * scale;
        let preview_h = img_h * scale;

        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(rgb(0x1e1e1e))
            .key_context("Preview")
            .on_action(cx.listener(|this, _: &crate::actions::CopyPreview, _, cx| {
                this.copy(cx);
            }))
            .on_action(cx.listener(|this, _: &crate::actions::SavePreview, window, cx| {
                this.save(window, cx);
            }))
            .on_action(cx.listener(|this, _: &crate::actions::ClosePreview, window, _| {
                this.close(window);
            }))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .px_4()
                    .py_3()
                    .bg(rgb(0x2d2d2d))
                    .child(
                        div()
                            .text_lg()
                            .text_color(rgb(0xffffff))
                            .child(tr(lang, MessageKey::PreviewTitle)),
                    )
                    .child(
                        div()
                            .flex()
                            .gap_2()
                            .child(btn(
                                "copy-btn",
                                tr(lang, MessageKey::PreviewCopyImage),
                                rgb(0x34C759),
                                cx,
                                |t, _, cx| t.copy(cx),
                            ))
                            .child(btn(
                                "save-btn",
                                tr(lang, MessageKey::PreviewSave),
                                rgb(0x007AFF),
                                cx,
                                |t, window, cx| t.save(window, cx),
                            ))
                            .child(
                                div()
                                    .id("close-btn")
                                    .px_4()
                                    .py_2()
                                    .rounded_md()
                                    .bg(rgb(0x555555))
                                    .text_color(rgb(0xffffff))
                                    .cursor_pointer()
                                    .text_sm()
                                    .child(tr(lang, MessageKey::PreviewClose))
                                    .on_click(cx.listener(|this, _, window, _| this.close(window))),
                            ),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .gap_3()
                    .p_4()
                    .child(
                        div()
                            .rounded_lg()
                            .overflow_hidden()
                            .border_1()
                            .border_color(rgb(0x444444))
                            .w(px(preview_w))
                            .h(px(preview_h))
                            .child(
                                canvas(
                                    move |_, _, _| {},
                                    move |bounds, _, window, cx| {
                                        if let Some(render_image) =
                                            gpui_image.use_render_image(window, cx)
                                        {
                                            let _ = window.paint_image(
                                                Bounds::new(bounds.origin, size(px(preview_w), px(preview_h))),
                                                gpui::Corners::default(),
                                                render_image,
                                                0,
                                                false,
                                            );
                                        }
                                    },
                                )
                                .size_full(),
                            ),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(rgb(0xaaaaaa))
                            .child(status),
                    ),
            )
    }
}

fn btn(
    id: &'static str,
    label: &'static str,
    bg: gpui::Rgba,
    cx: &mut Context<PreviewApp>,
    on_click: impl Fn(&mut PreviewApp, &mut Window, &mut Context<PreviewApp>) + 'static,
) -> impl IntoElement {
    div()
        .id(SharedString::from(id))
        .px_4()
        .py_2()
        .rounded_md()
        .bg(bg)
        .text_color(rgb(0xffffff))
        .cursor_pointer()
        .text_sm()
        .child(label.to_string())
        .on_click(cx.listener(move |this, _, window, cx| on_click(this, window, cx)))
}

pub fn open_preview(rgba: RgbaImage, cx: &mut App) {
    let w = rgba.width() as f32;
    let h = rgba.height() as f32;
    let win_w = w.min(1000.0) + 48.0;
    let win_h = h.min(800.0) + 140.0;
    let title = tr(i18n::language(cx), MessageKey::PreviewTitle).to_string();

    cx.open_window(
        WindowOptions {
            titlebar: Some(gpui::TitlebarOptions {
                title: Some(title.into()),
                ..Default::default()
            }),
            window_bounds: Some(WindowBounds::Windowed(Bounds::new(
                point(px(120.), px(80.)),
                size(px(win_w), px(win_h)),
            ))),
            window_background: WindowBackgroundAppearance::Opaque,
            focus: true,
            show: true,
            ..Default::default()
        },
        move |_, cx| cx.new(|cx| PreviewApp::new(rgba, cx)),
    )
    .ok();

    cx.bind_keys([
        KeyBinding::new("escape", crate::actions::ClosePreview, Some("Preview")),
        KeyBinding::new("ctrl-c", crate::actions::CopyPreview, Some("Preview")),
        KeyBinding::new("ctrl-s", crate::actions::SavePreview, Some("Preview")),
    ]);
}
